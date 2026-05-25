use crate::macros::macros::require_project_permission;
use crate::model::app_state::AppState;
use crate::model::overlay::WsBroadcast;
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
/// Reads the file from git and seeds the overlay base content for diff and broadcast.
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
    // new files the user just created locally wont exist at origin/{branch} yet,
    // so a read miss is expected. seed the overlay with empty base content; the
    // file will live as a "draft" in the project tree until its first commit.
    let content = git_service::read_file(
        std::path::Path::new(&state.repo_loc.join(proj_id.to_string())),
        branch.as_str(),
        std::path::Path::new(&file_name),
    )
    .await
    .unwrap_or_default();

    let overlay_tx = state.get_or_create_overlay(
        proj_id,
        file_name,
        user_id,
        content.clone(),
        branch,
    );

    // push the joining user onto the per-file channel so any subscriber that
    // connected before this PUT immediately picks them up in the active-editors
    // panel, instead of waiting for the new user to type their first keystroke.
    let _ = overlay_tx.send(WsBroadcast::Overlay {
        user_id,
        content,
        line_section: (0, 0),
    });

    // also refresh the project-wide activity snapshot so the board view and the
    // dashboard count reflect the new session.
    let activity_tx = state
        .repo_states
        .get(&proj_id)
        .map(|p| p.activity_tx.clone());
    if let Some(tx) = activity_tx {
        let _ = tx.send(state.compute_activity(&proj_id));
    }

    HttpResponse::Ok().finish()
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
