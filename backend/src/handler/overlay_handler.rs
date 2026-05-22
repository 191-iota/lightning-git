use crate::macros::macros::require_project_permission;
use crate::model::app_state::AppState;
use crate::model::overlay::extract_overlay;
use crate::model::user::MiddlewareData;
use crate::service::git_service;
use crate::service::overlay_service;
use actix_web::HttpResponse;
use actix_web::web;
use serde_json::json;
use uuid::Uuid;

/// One-shot snapshot of a file overlay: base content plus every active user's
/// in-flight content. Used by OverlayView to seed before the WS attaches.
#[utoipa::path(
    get,
    path = "/api/overlay/{proj_id}/{file_name}",
    params(
        ("proj_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("file_name" = String, Path, example = "main.rs"),
    ),
    tag = "overlay"
)]
pub async fn get_overlay(
    state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, String)>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let (proj_id, user_id, file_name) = path.into_inner();
    require_project_permission!(&state, &proj_id, &ext_data.user_id);

    // check if the project even exists before trying anything
    let Some(proj) = state.repo_states.get(&proj_id) else {
        return HttpResponse::NotFound().finish();
    };

    let Some(overlay_ref) = extract_overlay(&proj, &file_name) else {
        return HttpResponse::NotFound().finish();
    };

    let overlay_res = overlay_service::build_overlay_response(&overlay_ref, user_id).await;
    // NOTE: might want to filter out stale cursors here
    HttpResponse::Ok().json(overlay_res)
}

/// Open or re-open a user's overlay on a file at a given branch.
/// New files not yet on the remote are accepted with empty base content.
/// Returns 403 if the path is excluded by .gitignore or matches a sensitive pattern.
#[utoipa::path(
    put,
    path = "/api/overlay/{proj_id}/{user_id}/{file_name}",
    params(
        ("proj_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("user_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("file_name" = String, Path, example = "src/main.rs"),
        ("branch" = String, Query, example = "main"),
    ),
    tag = "overlay"
)]
pub async fn create_active_overlay(
    state: web::Data<AppState>,
    ext_data: web::ReqData<MiddlewareData>,
    path: web::Path<(Uuid, Uuid, String)>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> HttpResponse {
    let (proj_id, user_id, file_name) = path.into_inner();
    let branch = match query.get("branch") {
        Some(b) => b.clone(),
        None => return HttpResponse::BadRequest().body("missing branch query param"),
    };

    require_project_permission!(&state, &proj_id, &ext_data.user_id);

    let repo_path = state.repo_loc.join(proj_id.to_string());

    // never share secrets or known credential file types regardless of repo
    // .gitignore. this is the safety net for repos that arent careful.
    if is_sensitive_path(&file_name) {
        log::info!(
            "Refused overlay for sensitive path (proj {proj_id}, file {file_name})"
        );
        return HttpResponse::Forbidden().body("file is ignored by Lightning Git");
    }

    if git_service::is_ignored(&repo_path, &file_name).await {
        log::info!(
            "Refused overlay for gitignored path (proj {proj_id}, file {file_name})"
        );
        return HttpResponse::Forbidden().body("file is gitignored");
    }

    // new files that the user just created locally wont be on the remote yet,
    // git show will fail. seed the overlay with empty base content so the user
    // can still share live edits while they author the file
    let content = git_service::read_file(
        std::path::Path::new(&repo_path),
        branch.as_str(),
        std::path::Path::new(&file_name),
    )
    .await
    .unwrap_or_default();

    state.get_or_create_overlay(proj_id, file_name, user_id, content, branch);

    // broadcast so any subscribed web client sees the new session without waiting
    // for the user to type the first character
    let activity_tx = state
        .repo_states
        .get(&proj_id)
        .map(|p| p.activity_tx.clone());
    if let Some(tx) = activity_tx {
        let _ = tx.send(state.compute_activity(&proj_id));
    }

    HttpResponse::Ok().finish()
}

/// Hardcoded deny list for files that should never go through the overlay,
/// independent of repository .gitignore configuration.
fn is_sensitive_path(file_name: &str) -> bool {
    let last = file_name.rsplit('/').next().unwrap_or(file_name);
    let lower = last.to_ascii_lowercase();

    if lower == ".env" || lower.starts_with(".env.") || lower.ends_with(".env") {
        return true;
    }
    if lower == ".npmrc" || lower == ".netrc" || lower == ".git-credentials" {
        return true;
    }
    const SENSITIVE_SUFFIXES: &[&str] = &[
        ".pem", ".key", ".p12", ".pfx", ".crt", ".cert", ".keystore", ".jks",
    ];
    SENSITIVE_SUFFIXES.iter().any(|s| lower.ends_with(s))
}

/// Notbremse. Reset the caller's overlay in every file of the project back
/// to the committed branch state; caller is identified by the JWT.
#[utoipa::path(
    delete,
    path = "/api/overlay/me/{proj_id}",
    params(("proj_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    tag = "overlay",
)]
pub async fn wipe_my_overlay(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let proj_id = path.into_inner();
    require_project_permission!(&state, &proj_id, &ext_data.user_id);

    let reset = state.reset_user_overlays(&proj_id, &ext_data.user_id);
    log::info!(
        "Notbremse triggered: user {} reset {} overlay(s) in project {}",
        ext_data.user_id,
        reset,
        proj_id
    );
    HttpResponse::Ok().json(json!({ "reset": reset }))
}
