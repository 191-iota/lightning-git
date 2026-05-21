use crate::macros::macros::require_project_permission;
use crate::model::app_state::AppState;
use crate::model::overlay::extract_overlay;
use crate::model::user::MiddlewareData;
use crate::service::git_service;
use crate::service::overlay_service;
use actix_web::HttpResponse;
use actix_web::web;
use uuid::Uuid;

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
    let content = match git_service::read_file(
        std::path::Path::new(&state.repo_loc.join(proj_id.to_string())),
        branch.as_str(),
        std::path::Path::new(&file_name),
    )
    .await
    {
        Ok(c) => c,
        Err(e) => {
            log::error!(
                "Failed reading file for overlay (proj {proj_id}, branch {branch}, file {file_name}): {e}"
            );
            return HttpResponse::BadRequest().body("Branch or file not found");
        }
    };

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
