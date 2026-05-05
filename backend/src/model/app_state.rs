use std::path::PathBuf;

use dashmap::DashMap;
use dashmap::Entry;
use tokio::sync::broadcast;
use tokio::time::Instant;
use uuid::Uuid;

use crate::error::custom_errors::OverlayError;

use super::overlay::Overlay;
use super::overlay::OverlayWsMsg;
use super::overlay::ProjectLiveState;
use super::overlay::UserOverlay;

// So we do not need to re-init everytime we use it in handlers
#[derive(Clone)]
pub struct AppState {
    // Key: project id; Value: live state
    pub repo_states: DashMap<Uuid, ProjectLiveState>,
    pub repo_loc: PathBuf,
}

impl AppState {
    pub fn get_or_create_overlay(
        &self,
        project_id: Uuid,
        file_name: String,
        user_id: Uuid,
        initial_content: String,
        branch: String,
    ) -> broadcast::Sender<OverlayWsMsg> {
        let project_state =
            self.repo_states
                .entry(project_id)
                .or_insert_with(|| ProjectLiveState {
                    overlays: DashMap::new(),
                });
        let overlay = project_state.overlays.entry(file_name).or_insert_with(|| {
            let (tx, _rx) = broadcast::channel(16);
            Overlay {
                original_content: initial_content.clone(),
                user_contents: DashMap::new(),
                tx,
            }
        });
        let content_for_user = overlay.original_content.clone();

        match overlay.user_contents.entry(user_id) {
            Entry::Occupied(mut e) => {
                let uo = e.get_mut();
                uo.branch = branch;
                uo.updated_at = Instant::now();
            }
            Entry::Vacant(e) => {
                e.insert(UserOverlay::new(branch, content_for_user));
            }
        }
        overlay.tx.clone()
    }

    pub fn get_file_overlay(
        &self,
        project_id: Uuid,
        file_name: String,
    ) -> Result<Overlay, OverlayError> {
        let project = self.get_project_state(&project_id)?;

        project
            .overlays
            .get(&file_name)
            .map(|o| o.clone())
            .ok_or(OverlayError::FileOverlayNotFoundError(file_name))
    }

    pub fn get_project_state(
        &self,
        project_id: &Uuid,
    ) -> Result<dashmap::mapref::one::Ref<'_, Uuid, ProjectLiveState>, OverlayError> {
        self.repo_states
            .get(project_id)
            .ok_or(OverlayError::ProjectOverlayNotFoundError(
                project_id.to_string(),
            ))
    }
}
