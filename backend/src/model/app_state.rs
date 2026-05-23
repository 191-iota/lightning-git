use std::path::PathBuf;

use dashmap::DashMap;
use dashmap::Entry;
use supabase_auth::models::AuthClient;
use supabase_jwt::JwksCache;
use supabase_rs::SupabaseClient;
use tokio::sync::broadcast;
use tokio::time::Instant;
use uuid::Uuid;

use crate::error::custom_errors::OverlayError;

use super::overlay::ActiveEdit;
use super::overlay::Overlay;
use super::overlay::OverlayChangeReq;
use super::overlay::ProjectLiveState;
use super::overlay::UserOverlay;

// So we do not need to re-init everytime we use it in handlers
#[derive(Clone)]
pub struct AppState {
    // Key: project id; Value: live state
    pub repo_states: DashMap<Uuid, ProjectLiveState>,
    pub sb_client: SupabaseClient,
    pub repo_loc: PathBuf,
    pub auth_client: AuthClient,
    pub github_client_id: String,
    pub github_callback_url: String,
    pub github_client_secret: String,
    // shared JwksCache so the JWKS isnt re-fetched on every request
    pub jwks_cache: JwksCache,
}

impl AppState {
    pub fn get_or_create_overlay(
        &self,
        project_id: Uuid,
        file_name: String,
        user_id: Uuid,
        initial_content: String,
        branch: String,
    ) -> broadcast::Sender<OverlayChangeReq> {
        let project_state = self.repo_states.entry(project_id).or_insert_with(|| {
            let (activity_tx, _) = broadcast::channel(16);
            ProjectLiveState {
                overlays: DashMap::new(),
                activity_tx,
            }
        });
        let mut overlay = project_state.overlays.entry(file_name).or_insert_with(|| {
            let (tx, _rx) = broadcast::channel(16);
            Overlay {
                original_content: initial_content.clone(),
                user_contents: DashMap::new(),
                tx,
                comments: DashMap::new(),
            }
        });
        // always refresh the cached base with whatever git just gave us so
        // subsequent PUTs dont keep serving an outdated original_content.
        overlay.original_content = initial_content.clone();
        let content_for_user = overlay.original_content.clone();

        match overlay.user_contents.entry(user_id) {
            Entry::Occupied(mut e) => {
                // Treat every (re)create as a fresh session for this user on this
                // file: reset their content/divergence/edited region to the
                // freshly-read base. Otherwise stale typing from a prior session
                // makes the frontend paint every line as edited.
                let uo = e.get_mut();
                uo.branch = branch;
                uo.content = content_for_user.clone();
                uo.edited_sections = (0, 0);
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

    // Used by the project activity WS so subscribers can connect even before
    // any file overlay exists yet.
    pub fn ensure_project_state(&self, project_id: Uuid) -> broadcast::Sender<Vec<ActiveEdit>> {
        let entry = self.repo_states.entry(project_id).or_insert_with(|| {
            let (activity_tx, _) = broadcast::channel(16);
            ProjectLiveState {
                overlays: DashMap::new(),
                activity_tx,
            }
        });
        entry.activity_tx.clone()
    }

    // Refreshes the cached `original_content` for a file's overlay using a freshly
    // read base (typically read by the merge service, which already fetches). Lets
    // the cache stay in sync with main without spending an extra git call.
    pub fn refresh_overlay_base(&self, project_id: &Uuid, file_name: &str, new_base: String) {
        let Some(proj) = self.repo_states.get(project_id) else { return };
        let Some(mut overlay) = proj.overlays.get_mut(file_name) else { return };
        overlay.original_content = new_base;
    }

    // Notbremse: reset the user's in-flight overlay on every file back to the
    // committed branch state (the original_content cached when the overlay was
    // opened, which came from git show origin/{branch}:{path}). The user
    // remains in the session; their typing in flight is gone. We broadcast the
    // reset content on each per-file channel so teammates' views update, then
    // refresh the project-wide activity feed so the edited region drops to
    // zero in the active-editors panel.
    pub fn reset_user_overlays(&self, project_id: &Uuid, user_id: &Uuid) -> usize {
        let Some(proj) = self.repo_states.get(project_id) else {
            return 0;
        };
        let mut reset_count = 0usize;
        for overlay in proj.overlays.iter() {
            let base = overlay.original_content.clone();
            let tx = overlay.tx.clone();
            let mut touched = false;
            if let Some(mut uo) = overlay.user_contents.get_mut(user_id) {
                uo.content = base.clone();
                uo.edited_sections = (0, 0);
                uo.updated_at = Instant::now();
                touched = true;
            }
            if touched {
                reset_count += 1;
                let _ = tx.send(OverlayChangeReq {
                    user_id: *user_id,
                    content: base,
                    line_section: (0, 0),
                });
            }
        }
        let activity_tx = proj.activity_tx.clone();
        // release the read guard before we recompute the activity snapshot so
        // compute_activity can re-acquire the per-shard locks without contention
        drop(proj);
        let _ = activity_tx.send(self.compute_activity(project_id));
        reset_count
    }

    /// File paths held in any user's overlay for this project on the given branch.
    /// Used by the tree endpoint to surface drafts that are not yet committed.
    pub fn overlay_files_for_branch(&self, project_id: &Uuid, branch: &str) -> Vec<String> {
        let Some(proj) = self.repo_states.get(project_id) else { return Vec::new(); };
        let mut out: Vec<String> = Vec::new();
        for entry in proj.overlays.iter() {
            let on_branch = entry
                .value()
                .user_contents
                .iter()
                .any(|u| u.value().branch == branch);
            if on_branch {
                out.push(entry.key().clone());
            }
        }
        out
    }

    pub fn compute_activity(&self, project_id: &Uuid) -> Vec<ActiveEdit> {
        let mut edits = Vec::new();
        let Some(proj) = self.repo_states.get(project_id) else {
            return edits;
        };
        for entry in proj.overlays.iter() {
            let file = entry.key().clone();
            let overlay = entry.value();
            for user_entry in overlay.user_contents.iter() {
                let uo = user_entry.value();
                edits.push(ActiveEdit {
                    file: file.clone(),
                    user_id: *user_entry.key(),
                    branch: uo.branch.clone(),
                    edited_sections: uo.edited_sections,
                });
            }
        }
        edits
    }
}
