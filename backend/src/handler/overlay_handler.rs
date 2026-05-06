use crate::model::app_state::AppState;
use crate::model::overlay::extract_overlay;
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
) -> HttpResponse {
    let (proj_id, user_id, file_name) = path.into_inner();

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
    path = "/api/overlay/{proj_id}/{file_name}",
    params(
        ("proj_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("file_name" = String, Path, example = "main.rs"),
    ),
    tag = "overlay"
)]
pub async fn create_active_overlay(
    state: web::Data<AppState>,
    // <(proj_id, user_id, file_name, branch)>
    path: web::Path<(Uuid, Uuid, String, String)>,
) -> HttpResponse {
    // TODO: revisit errorhandling here
    let path = path.into_inner();
    let proj_id = path.0;
    let user_id = path.1;
    let file_name = path.2;
    let branch = path.3;

    // TODO: This should only create an overlay if the overlay for a file is not active

    let content = git_service::read_file(
        std::path::Path::new(&state.repo_loc.join(proj_id.to_string())),
        branch.as_str(),
        std::path::Path::new(&file_name),
    )
    .await
    .unwrap(); // TODO: this panics if the file doesn't exist on that branch, need to handle

    state.get_or_create_overlay(proj_id, file_name, user_id, content, branch);

    // previously returned Created but the frontend doesn't check status anyway
    HttpResponse::Ok().finish()
}
