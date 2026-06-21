use actix_web::web;

use uuid::Uuid;

use crate::error::custom_errors::OverlayError;
use crate::model::app_state::AppState;
use crate::model::overlay::Overlay;
use crate::model::overlay::OverlayViewRes;
use crate::model::overlay::UserOverlayRes;

/// Snapshot the live overlay state for one file into a serializable response.
/// Returns the caller's own content plus every other user's content separately.
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

/// Collect the live content of one file grouped by branch.
/// Used by the merge service to diff each branch's tip against main.
/// Each entry carries the user_id so downstream code can attribute the hunk
/// to its editor instead of just labeling it by branch.
pub fn extract_overlay_file_contents(
    file_name: String,
    project_id: Uuid,
    state: web::Data<AppState>,
) -> Result<Vec<OverlaySource>, OverlayError> {
    let mut result: Vec<OverlaySource> = Vec::new();
    let file_overlays = state.get_file_overlay(project_id, file_name)?;
    file_overlays.user_contents.iter().for_each(|v| {
        let user_overlay = v.value();
        result.push(OverlaySource {
            branch: user_overlay.branch.clone(),
            user_id: *v.key(),
            content: user_overlay.content.clone(),
        });
    });

    Ok(result)
}

pub struct OverlaySource {
    pub branch: String,
    pub user_id: Uuid,
    pub content: String,
}

// TODO: create functionality for automatic overlay pruning
