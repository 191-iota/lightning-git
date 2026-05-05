use std::path::PathBuf;

use dashmap::DashMap;
use uuid::Uuid;

use super::overlay::ProjectLiveState;

// So we do not need to re-init everytime we use it in handlers
#[derive(Clone)]
pub struct AppState {
    // Key: project id; Value: live state
    pub repo_states: DashMap<Uuid, ProjectLiveState>,
    pub repo_loc: PathBuf,
}
