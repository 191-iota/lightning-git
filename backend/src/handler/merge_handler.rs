use actix_web::HttpResponse;
use actix_web::web;

use crate::macros::macros::require_project_permission;
use crate::model::app_state::AppState;
use crate::model::overlay::extract_overlay;
use crate::model::user::MiddlewareData;
use crate::service::merge_service;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/overlay/merge/{proj_id}/{file_name}",
    params(
        ("proj_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("file_name" = String, Path, example = "main.rs"),
    ),
    tag = "merge"
)]
pub async fn get_merge_conflicts(
    state: web::Data<AppState>,
    path: web::Path<(Uuid, String)>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    let (proj_id, file_name) = path.into_inner();

    require_project_permission!(&state.sb_client, &proj_id, &ext_data.user_id);

    let Some(proj) = state.repo_states.get(&proj_id) else {
        return HttpResponse::NotFound().finish();
    };

    let Some(_overlay_ref) = extract_overlay(&proj, &file_name) else {
        return HttpResponse::NotFound().finish();
    };

    // TODO: rename because of bad naming
    // TODO: just pass in the file overlay instead of the whole appstate
    let conflicts = merge_service::calculate_live_diff(
        file_name,
        proj_id,
        state.clone(),
        std::path::Path::new(&state.repo_loc.join(proj_id.to_string())),
    )
    .await;

    match conflicts {
        Ok(v) => HttpResponse::Ok().json(v),

        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
