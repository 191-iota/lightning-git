use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
pub enum RepoError {
    #[error("Failed to read project dir {0}")]
    ProjectDirReadError(String),

    #[error("git_clone_repo failed (src {url})")]
    GitCloneError {
        url: String,
        #[source]
        source: Box<LGitIoError>,
    },

    #[error("Database insertion error: {0}")]
    InsertionError(String),

    #[error("Extraction error:{0}")]
    UpdateError(String),

    #[error("Extraction error:{0}")]
    ExtractionError(String),

    #[error("Deletion error:{0}")]
    DeletionError(String),

    #[error("Invalid url error: {0}")]
    InvalidUrl(String),

    #[error("Not found error: {0}")]
    NotFound(String),
}

#[derive(Error, Debug, Serialize)]
pub enum LGitIoError {
    #[error(transparent)]
    OverlayError(#[from] OverlayError),

    #[error(transparent)]
    GitError(#[from] GitError),

    #[error(transparent)]
    RepoError(#[from] RepoError),
}

#[derive(Error, Debug, Serialize)]
pub enum OverlayError {
    #[error("Failed retrieving project overlay")]
    ProjectOverlayNotFoundError(String),

    #[error("Failed retrieving file overlay")]
    FileOverlayNotFoundError(String),

    #[error("Failed retrieving user overlay")]
    UserOverlayNotFoundError(String),
}

#[derive(Error, Debug, Serialize)]
pub enum GitError {
    #[error("Project not found")]
    ProjectOverlayNotFoundError(String),
}
