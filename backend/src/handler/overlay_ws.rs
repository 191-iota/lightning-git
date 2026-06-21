use actix_web::HttpResponse;
use actix_web::web::Payload;
use actix_web::{
    HttpRequest,
    web::{self},
};
use actix_ws::Message;
use futures_util::StreamExt;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;
use uuid::Uuid;

use crate::model::app_state::AppState;
use crate::model::overlay::extract_overlay;
use crate::model::overlay::Comment;
use crate::model::overlay::UserOverlayRes;
use crate::model::overlay::WsBroadcast;
use crate::service::merge_service;

/// Project-wide activity WebSocket. Pushes a Vec<ActiveEdit> snapshot on
/// connect and on every per-file change anywhere in the project.
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
                    match event {
                        Ok(snapshot) => {
                            let text = serde_json::to_string(&snapshot).unwrap();
                            if session.text(text).await.is_err() {
                                break;
                            }
                        }
                        // dropping a message because the receiver fell behind is
                        // not a fatal error. previously this killed the WS for
                        // any user whose snapshot arrived during a busy moment,
                        // which manifested as "the second user never shows up".
                        Err(RecvError::Lagged(_)) => continue,
                        Err(RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    res
}

/// Per-file overlay WebSocket. Inbound: OverlayChangeReq updates from the
/// editing user. Outbound: every other subscriber's changes, self-filtered.
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

    // ensure the overlay exists so a viewer-only subscriber (e.g. a browser
    // tab on a file no one is actively editing) can attach the WS. then
    // capture the initial snapshot under read guards and drop them before
    // the WS upgrade so we dont hold DashMap locks across the await.
    let tx = state.ensure_file_overlay(project_id, &file_name);
    let (initial_snapshot, mut rx) = {
        let proj = match state.repo_states.get(&project_id) {
            Some(p) => p,
            None => return HttpResponse::InternalServerError().finish(),
        };
        let overlay_ref = match extract_overlay(&proj, &file_name) {
            Some(o) => o,
            None => return HttpResponse::InternalServerError().finish(),
        };
        let comments: Vec<Comment> = overlay_ref
            .comments
            .iter()
            .map(|e| e.value().clone())
            .collect();
        let all_user_contents: Vec<UserOverlayRes> = overlay_ref
            .user_contents
            .iter()
            .map(|entry| {
                let elapsed = entry.value().updated_at.elapsed();
                UserOverlayRes {
                    user_id: *entry.key(),
                    content: entry.value().content.clone(),
                    edited_sections: entry.value().edited_sections,
                    updated_at_secs: elapsed.as_secs(),
                    updated_at_nanos: elapsed.subsec_nanos(),
                }
            })
            .collect();
        (
            WsBroadcast::Snapshot {
                comments,
                all_user_contents,
            },
            tx.subscribe(),
        )
    };

    let (res, mut session, mut stream) = actix_ws::handle(&req, body).unwrap();

    let echo_self = req.query_string().contains("echo=true");

    // clones for the connect-time conflict seed computed inside the outbound
    // task. done after the WS upgrade and after every DashMap guard from the
    // snapshot block above has dropped, so calculate_live_diff's internal
    // write guard cannot deadlock against a read guard held across an await.
    let seed_state = state.clone();
    let seed_file = file_name.clone();
    let seed_project = project_id;

    // Server -> Client (Outbound)
    actix_web::rt::spawn(async move {
        // seed the client with the current comment list + active editors so
        // they dont need a separate HTTP fetch to populate the UI.
        if let Ok(text) = serde_json::to_string(&initial_snapshot) {
            if session.text(text).await.is_err() {
                return;
            }
        }
        // seed the predicted merge conflicts so the client renders them
        // immediately instead of polling GET /api/merge. on Err (e.g. file is
        // a draft not yet on main) we simply skip; no panic.
        let base = seed_state.repo_loc.join(seed_project.to_string());
        if let Ok(conflicts) = merge_service::calculate_live_diff(
            seed_file.clone(),
            seed_project,
            seed_state.clone(),
            &base,
        )
        .await
        {
            let msg = WsBroadcast::Conflicts {
                file: seed_file.clone(),
                conflicts,
            };
            if let Ok(text) = serde_json::to_string(&msg) {
                if session.text(text).await.is_err() {
                    return;
                }
            }
        }
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    // self-filter overlay echoes only; comment events must
                    // reach the originating client too so it learns the
                    // server-assigned id and timestamp.
                    let skip = !echo_self
                        && match &msg {
                            WsBroadcast::Overlay { user_id: u, .. } => *u == user_id,
                            _ => false,
                        };
                    if skip {
                        continue;
                    }
                    let text = serde_json::to_string(&msg).unwrap();
                    if session.text(text).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    });

    // Client -> Server (Inbound)
    let state_clone = state.clone();
    let file_name_clone = file_name.clone();
    actix_web::rt::spawn(async move {
        while let Some(Ok(Message::Text(content))) = stream.next().await {
            let Ok(incoming) = serde_json::from_str::<WsBroadcast>(&content) else {
                continue;
            };
            let (broadcast, refresh_activity) =
                process_incoming(&state_clone, &project_id, &file_name_clone, user_id, incoming);
            if let Some(broadcast) = broadcast {
                if let Some(proj) = state_clone.repo_states.get(&project_id) {
                    if let Some(overlay) = proj.overlays.get(&file_name_clone) {
                        let _ = overlay.tx.send(broadcast);
                    }
                }
            }
            if refresh_activity {
                let activity_tx = state_clone
                    .repo_states
                    .get(&project_id)
                    .map(|p| p.activity_tx.clone());
                if let Some(tx) = activity_tx {
                    let _ = tx.send(state_clone.compute_activity(&project_id));
                }

                // refresh_activity is true only for Overlay edits, which is
                // exactly when conflicts can change. recompute and broadcast
                // the fresh conflict set to every subscriber so clients no
                // longer poll GET /api/merge.
                //
                // calculate_live_diff is awaited OUTSIDE any repo_states /
                // overlays guard: it internally takes a write guard via
                // refresh_overlay_base, so holding a read guard across this
                // await would deadlock (the same trap get_merge_conflicts
                // documented). only after it returns do we re-acquire the
                // overlay in a fresh short scope to send. Conflicts is not
                // self-filtered, so the editing client also receives it.
                let base = state_clone.repo_loc.join(project_id.to_string());
                if let Ok(conflicts) = merge_service::calculate_live_diff(
                    file_name_clone.clone(),
                    project_id,
                    state_clone.clone(),
                    &base,
                )
                .await
                {
                    if let Some(proj) = state_clone.repo_states.get(&project_id) {
                        if let Some(overlay) = proj.overlays.get(&file_name_clone) {
                            let _ = overlay.tx.send(WsBroadcast::Conflicts {
                                file: file_name_clone.clone(),
                                conflicts,
                            });
                        }
                    }
                }
            }
        }
        // stream ended. intentionally DON'T remove user_contents here —
        // the same user_id often has multiple concurrent connections
        // (web viewer + VSC editor at minimum), and removing on the
        // first WS close wipes data the user's other sessions are still
        // actively using. entries get refreshed by the next createOverlay
        // call, or wiped explicitly via /api/overlay/me (Notbremse).
    });

    res
}

/// Apply an incoming WsBroadcast to overlay state and decide what to do
/// next. Returns the message to rebroadcast (None for ignored variants and
/// unauthorized comment deletions) and whether the activity snapshot should
/// be recomputed.
fn process_incoming(
    state: &AppState,
    project_id: &Uuid,
    file_name: &str,
    sender_id: Uuid,
    msg: WsBroadcast,
) -> (Option<WsBroadcast>, bool) {
    match msg {
        WsBroadcast::Overlay {
            user_id: _,
            content,
            line_section,
        } => {
            if let Some(proj) = state.repo_states.get(project_id) {
                if let Some(overlay) = proj.overlays.get(file_name) {
                    if let Some(mut user_overlay) = overlay.user_contents.get_mut(&sender_id) {
                        user_overlay.content = content.clone();
                        user_overlay.edited_sections = (line_section.0, line_section.1);
                        user_overlay.updated_at = Instant::now();
                    }
                }
            }
            (
                Some(WsBroadcast::Overlay {
                    user_id: sender_id,
                    content,
                    line_section,
                }),
                true,
            )
        }
        WsBroadcast::CommentCreated { line, text, .. } => {
            if text.trim().is_empty() {
                return (None, false);
            }
            let id = Uuid::new_v4();
            let created_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let comment = crate::model::overlay::Comment {
                id,
                user_id: sender_id,
                line,
                text: text.clone(),
                created_at,
            };
            if let Some(proj) = state.repo_states.get(project_id) {
                if let Some(overlay) = proj.overlays.get(file_name) {
                    overlay.comments.insert(id, comment);
                }
            }
            (
                Some(WsBroadcast::CommentCreated {
                    id,
                    user_id: sender_id,
                    line,
                    text,
                    created_at,
                }),
                false,
            )
        }
        WsBroadcast::CommentDeleted { id } => {
            // owner-only: only the comment author may delete it.
            let allowed = state
                .repo_states
                .get(project_id)
                .and_then(|proj| {
                    proj.overlays
                        .get(file_name)
                        .and_then(|ov| ov.comments.get(&id).map(|c| c.user_id == sender_id))
                })
                .unwrap_or(false);
            if !allowed {
                return (None, false);
            }
            if let Some(proj) = state.repo_states.get(project_id) {
                if let Some(overlay) = proj.overlays.get(file_name) {
                    overlay.comments.remove(&id);
                }
            }
            (Some(WsBroadcast::CommentDeleted { id }), false)
        }
        WsBroadcast::Snapshot { .. } => (None, false),
        // server -> client only; ignore if a client ever sends it.
        WsBroadcast::Conflicts { .. } => (None, false),
    }
}

