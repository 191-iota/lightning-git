use std::path::PathBuf;

// So we do not need to re-init everytime we use it in handlers
#[derive(Clone)]
pub struct AppState {
    pub repo_loc: PathBuf,
}
