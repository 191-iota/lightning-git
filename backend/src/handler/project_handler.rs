use crate::model::app_state::AppState;
use crate::model::project::CreateProjectReq;
use crate::model::project::CreateProjectRes;
use crate::model::project::FileReadReq;
use crate::service::git_service;
use actix_web::HttpResponse;
use actix_web::http::header::CONTENT_TYPE;
use actix_web::web;
use log::error;
use uuid::Uuid;
use validator::Validate;

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
) -> HttpResponse {
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().json(e);
    }

    let proj_id = Uuid::new_v4();
    let repo_path = state.repo_loc.join(proj_id.to_string());

    if let Err(e) = git_service::clone_repo(&req.repo_url, &repo_path).await {
        error!(
            "git_clone_repo failed (proj {proj_id}, src {}): {e}",
            &req.repo_url
        );
        return HttpResponse::BadRequest().body(format!(
            "Failed cloning project {}, consider checking permissions.",
            &req.repo_url,
        ));
    }

    HttpResponse::Ok().json(CreateProjectRes { proj_id })
}

#[utoipa::path(
    post,
    path = "/api/projects/{id}/{branch}/file",
    params(
        ("id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
    ),
    request_body = FileReadReq,
    tag = "project",
)]
pub async fn get_project_file(
    state: web::Data<AppState>,
    // <(project_id, branch)>
    path: web::Path<(Uuid, String)>,
    req: web::Json<FileReadReq>,
) -> HttpResponse {
    let path = path.into_inner();
    let proj_id = path.0;
    let branch = path.1;
    // Explicitly using fully qualified name (std::...) to avoid conflict with web::Path
    let content = git_service::read_file(
        std::path::Path::new(&state.repo_loc.join(proj_id.to_string())),
        branch.as_str(),
        std::path::Path::new(&req.file_path),
    )
    .await;

    HttpResponse::Ok()
        .insert_header((CONTENT_TYPE, "text/plain; charset=utf-8"))
        .body(content.unwrap())
}
