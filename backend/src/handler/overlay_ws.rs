use actix_web::HttpResponse;
use actix_web::web::Payload;
use actix_web::{
    HttpRequest,
    web::{self},
};
use actix_ws::Message;
use uuid::Uuid;

use crate::model::app_state::AppState;

pub async fn ws_overlay_stream(
    req: HttpRequest,
    body: Payload,
    state: web::Data<AppState>,
    // <(project_id, user_id)>
    path: web::Path<(Uuid, String)>,
) -> HttpResponse {
    let path = path.into_inner();
    let project_id = path.0;
    let user_id = path.1;
    let file_name = path.2;
    // TODO: Check permissions!!
}
