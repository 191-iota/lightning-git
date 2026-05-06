use actix_web::HttpResponse;
use actix_web::web::Payload;
use actix_web::{
    HttpRequest,
    web::{self},
};
use actix_ws::Message;
use futures_util::StreamExt;
use tokio::time::Instant;
use uuid::Uuid;

use crate::model::app_state::AppState;
use crate::model::overlay::extract_overlay;
use crate::model::overlay::{OverlayChangeReq, OverlayWsMsg};

#[utoipa::path(
    get,
    path = "/api/ws/{project_id}/{user_id}/{file_name}",
    params(
        ("project_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("user_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6"),
        ("file_name" = String, Path, example = "main.rs"),
    ),
    tag = "overlay"
)]
pub async fn ws_overlay_stream(
    req: HttpRequest,
    body: Payload,
    state: web::Data<AppState>,
    // <(project_id, user_id, file_name)>
    path: web::Path<(Uuid, Uuid, String)>,
) -> HttpResponse {
    let path = path.into_inner();
    let project_id = path.0;
    let user_id = path.1;
    let file_name = path.2;
    // TODO: Check permissions!!
    let Some(proj) = state.repo_states.get(&project_id) else {
        return HttpResponse::NotFound().finish();
    };

    let Some(overlay_ref) = extract_overlay(&proj, &file_name) else {
        return HttpResponse::NotFound().finish();
    };

    // Websocket handshake
    let (res, mut session, mut stream) = actix_ws::handle(&req, body).unwrap();

    let mut rx = overlay_ref.tx.subscribe();

    // Server -> Client (Outbound)
    actix_web::rt::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // skip sending user's own changes back to them
            if let OverlayWsMsg::Change(ref change) = msg
                && change.user_id == user_id
            {
                continue;
            }

            let text = serde_json::to_string(&msg).unwrap();
            if session.text(text).await.is_err() {
                break;
            }
        }
    });
    let _overlay_clone = overlay_ref.clone();

    // Client -> Server (Inbound)
    let state_clone = state.clone();
    let file_name_clone = file_name.clone();
    actix_web::rt::spawn(async move {
        while let Some(Ok(Message::Text(content))) = stream.next().await {
            // DashMap nesting: each layer can fail independently, can't collapse without skipping broadcast
            #[allow(clippy::collapsible_if)]
            if let Ok(change) = serde_json::from_str::<OverlayChangeReq>(&content) {
                if let Some(proj) = state_clone.repo_states.get(&project_id) {
                    if let Some(overlay) = proj.overlays.get(&file_name_clone) {
                        if let Some(mut user_overlay) = overlay.user_contents.get_mut(&user_id) {
                            user_overlay.content = change.content.clone();
                            user_overlay.edited_sections =
                                (change.line_section.0, change.line_section.1);
                            user_overlay.updated_at = Instant::now();
                        }
                        let _ = overlay.tx.send(OverlayWsMsg::Change(change));
                    }
                }
            }
        }
    });

    res
}
