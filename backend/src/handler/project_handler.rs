use std::collections::HashMap;

use crate::macros::macros::require_org_permission;
use crate::macros::macros::require_project_admin;
use crate::macros::macros::require_project_permission;
use crate::model::app_state::AppState;
use crate::model::project::AddProjectMemberReq;
use crate::model::project::CreateProjectReq;
use crate::model::project::CreateProjectRes;
use crate::model::project::ProjectRole;
use crate::model::project::ProjectTreeRes;
use crate::model::project::UpdateProjectReq;
use crate::model::user::MiddlewareData;
use crate::model::org::OrgRole;
use crate::repository::project_repository;
use crate::repository::user_repository;
use crate::service::git_service;
use crate::service::permission_service;
use crate::service::project_service;
use crate::service::project_service::inject_token;
use actix_web::HttpResponse;
use actix_web::http::header::CONTENT_TYPE;
use actix_web::web;
use log::error;
use log::warn;
use uuid::Uuid;
use validator::Validate;

/// Create a project under an org, clone the repo, and seed the creator as admin.
/// Optionally also derives one Kanban task per existing remote branch.
#[utoipa::path(
    post,
    path = "/api/projects",
    request_body = CreateProjectReq,
    responses(
        (status = 200, body = CreateProjectRes),
        (status = 500, description = "Internal server error"),
    ),
    tag = "project",
)]
pub async fn create_project(
    state: web::Data<AppState>,
    req: web::Json<CreateProjectReq>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(e);
    }

    // Caller must belong to the org they're creating the project under.
    require_org_permission!(&state, &req.org_id, &ext_data.user_id);

    let proj_id = Uuid::new_v4();
    let repo_path = state.repo_loc.join(proj_id.to_string());

    let clone_url =
        match user_repository::get_access_token(&state.sb_client, &ext_data.user_id).await {
            Ok(Some(token)) => inject_token(&req.repo_url, &token),
            Ok(None) => {
                // No stored GitHub token: the user has not finished "Authorize on
                // GitHub" yet, so a private repo will fail to clone below.
                warn!(
                    "No GitHub token for user {}, cloning {} unauthenticated (private repos will fail)",
                    &ext_data.user_id, &req.repo_url
                );
                req.repo_url.clone()
            }
            Err(e) => {
                error!("Failed reading GitHub token for user {}: {e}", &ext_data.user_id);
                req.repo_url.clone()
            }
        };

    if let Err(e) = git_service::clone_repo(&clone_url, &repo_path).await {
        error!(
            "git_clone_repo failed (proj {proj_id}, src {}): {e}",
            &req.repo_url
        );
        return HttpResponse::BadRequest().body(format!(
            "Failed cloning project {}, consider checking permissions.",
            &req.repo_url,
        ));
    }

    let res = project_repository::save_project(
        &state.sb_client,
        &proj_id,
        &req.name,
        &req.repo_url,
        &req.org_id,
        &ext_data.user_id,
    )
    .await;

    if let Err(e) = res {
        error!("Failed creating project: {e}");
        if let Err(cleanup_err) = tokio::fs::remove_dir_all(&repo_path).await {
            error!(
                "Failed cleaning up repo at {}: {cleanup_err}",
                repo_path.display()
            );
        }
        return HttpResponse::BadRequest().body("Failed processing request");
    }

    if req.create_tasks_retroactively {
        if let Err(e) =
            project_service::detect_and_create_tasks(&repo_path, &state.sb_client, &proj_id).await
        {
            error!("Failed creating project: {e}");
            return HttpResponse::BadRequest().finish();
        }
    }
    HttpResponse::Ok().json(CreateProjectRes { proj_id })
}

/// List a project's members with display names and roles.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/members",
    params(
        ("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
    ),
    tag = "project",
)]
pub async fn get_project_members(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let proj_id = path.into_inner();

    require_project_permission!(&state, &proj_id, &ext_data.user_id);

    let res = project_repository::get_project_members_full(&state.sb_client, &proj_id).await;
    match res {
        Ok(members) => HttpResponse::Ok().json(members),
        Err(e) => {
            error!("Failed getting project members: {e}");
            HttpResponse::BadRequest().body("Failed processing request")
        }
    }
}

/// Add a member to the project with a chosen role. Project admin only.
/// Rejects duplicates with 409.
#[utoipa::path(
    post,
    path = "/api/projects/{id}/members",
    params(("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    request_body = AddProjectMemberReq,
    tag = "project",
)]
pub async fn add_project_member(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: web::Json<AddProjectMemberReq>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(e);
    }
    let proj_id = path.into_inner();
    require_project_admin!(&state, &proj_id, &ext_data.user_id);

    let req = req.into_inner();

    // a project member must already belong to the project's org. otherwise
    // we'd let an admin invite arbitrary users who never accepted the org.
    let project_row = match project_repository::find_project_by_id(
        &state.sb_client,
        proj_id.to_string(),
    )
    .await
    {
        Ok(row) => row,
        Err(e) => {
            error!("Failed loading project for org-membership check: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let org_id = match project_row
        .get("org_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => {
            error!("Project {proj_id} has no resolvable org_id");
            return HttpResponse::InternalServerError().finish();
        }
    };
    match permission_service::check_org_permission(
        &state,
        &org_id,
        &req.user_id,
        OrgRole::Member,
    )
    .await
    {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::BadRequest()
                .body("User must already be a member of this org");
        }
        Err(e) => {
            error!("Failed checking candidate org membership: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    match project_repository::get_project_member_role(&state.sb_client, &proj_id, &req.user_id)
        .await
    {
        Ok(Some(_)) => return HttpResponse::Conflict().body("User is already a project member"),
        Ok(None) => {}
        Err(e) => {
            error!("Failed checking existing project member: {e}");
            return HttpResponse::InternalServerError().finish();
        }
    }

    match project_repository::add_user_to_project(
        &state.sb_client,
        &proj_id,
        &req.user_id,
        req.role,
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("Failed adding project member: {e}");
            HttpResponse::BadRequest().body("Failed adding project member")
        }
    }
}

/// Remove a member from the project. Project admin only.
/// Refuses to remove the last admin; transfer admin rights first.
#[utoipa::path(
    delete,
    path = "/api/projects/{id}/members/{user_id}",
    params(
        ("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("user_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
    ),
    tag = "project",
)]
pub async fn remove_project_member(
    state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let (proj_id, target_user) = path.into_inner();
    require_project_admin!(&state, &proj_id, &ext_data.user_id);

    let target_role =
        match project_repository::get_project_member_role(&state.sb_client, &proj_id, &target_user)
            .await
        {
            Ok(Some(role)) => role,
            Ok(None) => return HttpResponse::NotFound().body("Member not in project"),
            Err(e) => {
                error!("Failed reading project member role: {e}");
                return HttpResponse::InternalServerError().finish();
            }
        };

    if target_user == ext_data.user_id && matches!(target_role, ProjectRole::Admin) {
        let admin_count =
            match project_repository::count_project_admins(&state.sb_client, &proj_id).await {
                Ok(n) => n,
                Err(e) => {
                    error!("Failed counting admins: {e}");
                    return HttpResponse::InternalServerError().finish();
                }
            };
        if admin_count <= 1 {
            return HttpResponse::BadRequest()
                .body("Cannot remove the last admin; delete the project or promote another admin first");
        }
    }

    match project_repository::remove_user_from_project(&state.sb_client, &proj_id, &target_user)
        .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("Failed removing project member: {e}");
            HttpResponse::BadRequest().body("Failed removing project member")
        }
    }
}

/// Rename a project and add any new members listed in the body. Admin only.
#[utoipa::path(
    put,
    path = "/api/projects/{id}",
    params(
        ("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
    ),
    request_body = UpdateProjectReq,
    tag = "project",
)]
pub async fn update_project(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    req: web::Json<UpdateProjectReq>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(e);
    }
    let proj_id = path.into_inner();

    require_project_admin!(&state, &proj_id, &ext_data.user_id);

    let res =
        project_repository::update_project(&state.sb_client, &proj_id, req.into_inner()).await;
    match res {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("Failed updating project: {e}");
            HttpResponse::BadRequest().body("Failed processing request")
        }
    }
}

/// Delete a project, remove the local clone, and cascade tasks. Admin only.
#[utoipa::path(
    get,
    path = "/api/projects/{id}",
    params(
        ("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
    ),
    tag = "project",
)]
pub async fn delete_project(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let proj_id = path.into_inner();

    require_project_admin!(&state, &proj_id, &ext_data.user_id);
    let dest_path = state.repo_loc.join(proj_id.to_string());

    if let Err(e) = git_service::delete_repo(&dest_path).await {
        error!("Failed deleting repository for {proj_id}: {e}");
        return HttpResponse::BadRequest()
            .content_type("text/plain; charset=utf-8")
            .body("Unable to delete repo");
    }

    let res = project_repository::delete_project(&state.sb_client, proj_id.to_string()).await;
    match res {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_e) => HttpResponse::BadRequest().body("Failed deleting project"),
    }
}

/// Fetch project metadata and refresh remote refs.
/// Also re-derives Kanban tasks from current remote branches so the board stays in sync.
#[utoipa::path(
    get,
    path = "/api/projects/{id}",
    params(
        ("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
    ),
    tag = "project",
)]
pub async fn get_project(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let proj_id = path.into_inner();

    require_project_permission!(&state, &proj_id, &ext_data.user_id);
    let res = project_repository::find_project_by_id(&state.sb_client, proj_id.to_string()).await;

    let repo_path = state.repo_loc.join(proj_id.to_string());

    // Refresh remote-tracking refs so task detection and subsequent reads (via
    // git show origin/{branch}:{path}) see latest remote state. Fetch failure
    // shouldn't block the response
    if let Err(e) = git_service::fetch(&repo_path).await {
        error!("git fetch failed for proj {proj_id}: {e}");
    }

    // TODO: Should getting a project really fail when detecting and creating tasks fails?
    if let Err(e) =
        project_service::detect_and_create_tasks(&repo_path, &state.sb_client, &proj_id).await
    {
        error!("Task detection failed. proj_id : {}, error: {e}", proj_id);
        return HttpResponse::BadRequest().body("Task detection failed");
    }

    match res {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::BadRequest().body(format!("Failed getting project {e}")),
    }
}

/// HTTP snapshot of who is currently editing what in this project.
/// The WS endpoint /activity/ws emits the same shape live; this is for one-shot polls.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/activity",
    params(
        ("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
    ),
    tag = "project",
)]
pub async fn get_project_activity(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let proj_id = path.into_inner();
    require_project_permission!(&state, &proj_id, &ext_data.user_id);
    HttpResponse::Ok().json(state.compute_activity(&proj_id))
}

/// Sorted list of remote-tracking branch names for the project's clone.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/branches",
    params(("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    tag = "project",
)]
pub async fn list_project_branches(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let proj_id = path.into_inner();
    require_project_permission!(&state, &proj_id, &ext_data.user_id);

    let repo_path = state.repo_loc.join(proj_id.to_string());
    match git_service::list_remote_branches(&repo_path).await {
        Ok(set) => {
            let mut v: Vec<String> = set.into_iter().collect();
            v.sort();
            HttpResponse::Ok().json(v)
        }
        Err(e) => {
            error!("Failed listing branches for {proj_id}: {e}");
            HttpResponse::BadRequest().body("Failed listing branches")
        }
    }
}

/// File listing for the OverlayView tree: committed files at origin/{branch}
/// plus live-overlay drafts that haven't been committed yet.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/tree",
    params(
        ("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("branch" = String, Query, example = "main"),
    ),
    tag = "project",
)]
pub async fn list_project_tree(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<std::collections::HashMap<String, String>>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let proj_id = path.into_inner();
    require_project_permission!(&state, &proj_id, &ext_data.user_id);

    let branch = match query.get("branch") {
        Some(b) => b.clone(),
        None => return HttpResponse::BadRequest().body("missing branch query param"),
    };

    let repo_path = state.repo_loc.join(proj_id.to_string());
    let mut committed = match git_service::list_files(&repo_path, &branch).await {
        Ok(files) => files,
        Err(e) => {
            // a branch may only exist as live drafts on a not-yet-pushed local
            // branch; in that case ls-tree fails but we still want to return
            // the drafts. log the underlying git error and fall through.
            error!("list_files failed for {proj_id}@{branch}: {e}");
            Vec::new()
        }
    };
    committed.sort();

    let committed_set: std::collections::HashSet<&String> = committed.iter().collect();
    let mut drafts: Vec<String> = state
        .overlay_files_for_branch(&proj_id, &branch)
        .into_iter()
        .filter(|f| !committed_set.contains(f))
        .collect();
    drafts.sort();

    HttpResponse::Ok().json(ProjectTreeRes { committed, drafts })
}

/// Read a single file's text content from a given branch, served as text/plain.
#[utoipa::path(
    get,
    path = "/api/projects/{id}/file",
    params(
        ("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("branch" = String, Query, example = "main"),
        ("path" = String, Query, example = "src/main.rs"),
    ),
    tag = "project",
)]
pub async fn get_project_file(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<HashMap<String, String>>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let proj_id = path.into_inner();
    require_project_permission!(&state, &proj_id, &ext_data.user_id);

    let branch = match query.get("branch") {
        Some(b) => b.clone(),
        None => return HttpResponse::BadRequest().body("missing branch query param"),
    };
    let file_path = match query.get("path") {
        Some(p) => p.clone(),
        None => return HttpResponse::BadRequest().body("missing path query param"),
    };

    match git_service::read_file(
        &state.repo_loc.join(proj_id.to_string()),
        &branch,
        std::path::Path::new(&file_path),
    )
    .await
    {
        Ok(content) => HttpResponse::Ok()
            .insert_header((CONTENT_TYPE, "text/plain; charset=utf-8"))
            .body(content),
        Err(e) => {
            // file is a draft on this branch (no committed content yet).
            // return empty body so the OverlayView can still attach its
            // per-file WS and stream in live edits from teammates.
            error!("read_file miss for {file_path}@{branch} in proj {proj_id}: {e}");
            HttpResponse::Ok()
                .insert_header((CONTENT_TYPE, "text/plain; charset=utf-8"))
                .body("")
        }
    }
}
