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
use crate::model::overlay::OverlayChangeReq;

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/activity/ws",
    params(("project_id" = Uuid, Path, example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")),
    tag = "project"
)]
pub async fn ws_project_activity(
    req: HttpRequest,
    body: Payload,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> HttpResponse {
    let project_id = path.into_inner();
    let activity_tx = state.ensure_project_state(project_id);

    let (res, mut session, mut stream) = actix_ws::handle(&req, body).unwrap();

    let initial = state.compute_activity(&project_id);
    let initial_text = serde_json::to_string(&initial).unwrap();

    actix_web::rt::spawn(async move {
        if session.text(initial_text).await.is_err() {
            return;
        }
        let mut rx = activity_tx.subscribe();
        loop {
            tokio::select! {
                msg = stream.next() => {
                    match msg {
                        Some(Ok(Message::Ping(bytes))) => {
                            if session.pong(&bytes).await.is_err() { break; }
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => continue,
                    }
                }
                event = rx.recv() => {
                    let Ok(snapshot) = event else { break };
                    let text = serde_json::to_string(&snapshot).unwrap();
                    if session.text(text).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    res
}

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
    let Some(proj) = state.repo_states.get(&project_id) else {
        return HttpResponse::NotFound().finish();
    };
    let Some(overlay_ref) = extract_overlay(&proj, &file_name) else {
        return HttpResponse::NotFound().finish();
    };

    // Websocket handshake
    let (res, mut session, mut stream) = actix_ws::handle(&req, body).unwrap();

    let mut rx = overlay_ref.tx.subscribe();

    // viewers (web frontend) can opt in to see their own changes echoed back,
    // editors (vscode) keep the default self-filter so typing doesnt loop
    let echo_self = req.query_string().contains("echo=true");

    // Server -> Client (Outbound)
    actix_web::rt::spawn(async move {
        while let Ok(change) = rx.recv().await {
            if change.user_id == user_id && !echo_self {
                continue;
            }
            let text = serde_json::to_string(&change).unwrap();
            if session.text(text).await.is_err() {
                break;
            }
        }
    });

    // Client -> Server (Inbound)
    let state_clone = state.clone();
    let file_name_clone = file_name.clone();
    actix_web::rt::spawn(async move {
        while let Some(Ok(Message::Text(content))) = stream.next().await {
            // DashMap nesting: each layer can fail independently, can't collapse without skipping broadcast
            #[allow(clippy::collapsible_if)]
            if let Ok(change) = serde_json::from_str::<OverlayChangeReq>(&content) {
                let activity_tx = if let Some(proj) = state_clone.repo_states.get(&project_id) {
                    if let Some(overlay) = proj.overlays.get(&file_name_clone) {
                        if let Some(mut user_overlay) = overlay.user_contents.get_mut(&user_id) {
                            user_overlay.content = change.content.clone();
                            user_overlay.edited_sections =
                                (change.line_section.0, change.line_section.1);
                            user_overlay.updated_at = Instant::now();
                        }
                        let _ = overlay.tx.send(change);
                    }
                    Some(proj.activity_tx.clone())
                } else {
                    None
                };
                // released the proj guard before recomputing to avoid lock re-entry
                if let Some(tx) = activity_tx {
                    let _ = tx.send(state_clone.compute_activity(&project_id));
                }
            }
        }
    });

    res
}
