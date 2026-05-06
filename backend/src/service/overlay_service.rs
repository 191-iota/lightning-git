use actix_web::web;

use uuid::Uuid;

use crate::error::custom_errors::OverlayError;
use crate::model::app_state::AppState;
use crate::model::overlay::Overlay;
use crate::model::overlay::OverlayViewRes;
use crate::model::overlay::UserOverlayRes;

/// Takes a reference to an `Overlay`, performs the async lock read,
/// snapshots the dash‑maps and returns a fully‑serialisable `OverlayResponse`.
pub async fn build_overlay_response(overlay: &Overlay, user_id: Uuid) -> OverlayViewRes {
    let user_content = overlay
        .user_contents
        .get(&user_id)
        .map(|u| u.content.clone())
        .unwrap_or_else(|| overlay.original_content.clone());

    let all_user_contents: Vec<UserOverlayRes> = overlay
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

    OverlayViewRes {
        content: user_content,
        original_content: overlay.original_content.clone(),
        all_user_contents,
    }
}

// PERF: revisit clone, probably unnecessary
pub fn extract_overlay_file_contents(
    file_name: String,
    project_id: Uuid,
    state: web::Data<AppState>,
    // Vec<Branchname, Contents>
) -> Result<Vec<(String, String)>, OverlayError> {
    let mut result: Vec<(String, String)> = Vec::new();
    let file_overlays = state.get_file_overlay(project_id, file_name)?;
    file_overlays.user_contents.iter().for_each(|v| {
        let user_overlay = v.value();

        result.push((user_overlay.branch.clone(), user_overlay.content.clone()));
    });

    Ok(result)
}

// TODO: create functionality for automatic overlay pruning
