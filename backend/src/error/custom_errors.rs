use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
pub enum RepoError {
    #[error("Database insertion error: {0}")]
    InsertionError(String),

    #[error("Extraction error:{0}")]
    UpdateError(String),

    #[error("Extraction error:{0}")]
    ExtractionError(String),

    #[error("Deletion error:{0}")]
    DeletionError(String),

    #[error("Not found error: {0}")]
    NotFound(String),
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Email already registered")]
    EmailAlreadyExists,

    #[error("Username already taken")]
    UsernameAlreadyTaken,

    #[error("Already exists")]
    AlreadyExists,

    #[error("Password too weak")]
    WeakPassword,

    #[error("Too many attempts")]
    RateLimited,

    #[error("Email not confirmed")]
    EmailNotConfirmed,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Authentication failed: {0}")]
    Other(String),
}

impl AuthError {
    /// Classifies a supabase-auth error into an AuthError variant by examining the error message.
    /// This provides a more structured way to handle errors than direct string matching in handlers.
    pub fn from_supabase_error(error: &impl std::fmt::Display) -> Self {
        let err_msg = error.to_string();
        let err_lower = err_msg.to_lowercase();

        // Pattern match based on error message content
        // Order matters: more specific patterns should come first
        if err_lower.contains("already") && err_lower.contains("email") {
            AuthError::EmailAlreadyExists
        } else if err_lower.contains("already") && err_lower.contains("username") {
            AuthError::UsernameAlreadyTaken
        } else if err_lower.contains("already exists") {
            AuthError::AlreadyExists
        } else if err_lower.contains("password")
            && (err_lower.contains("weak")
                || err_lower.contains("strength")
                || err_lower.contains("requirement"))
        {
            AuthError::WeakPassword
        } else if err_lower.contains("rate") || err_lower.contains("too many") {
            AuthError::RateLimited
        } else if err_lower.contains("not confirmed") {
            AuthError::EmailNotConfirmed
        } else if err_lower.contains("invalid")
            || err_lower.contains("credentials")
            || err_lower.contains("wrong")
        {
            AuthError::InvalidCredentials
        } else {
            AuthError::Other(err_msg)
        }
    }
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
}

#[derive(Error, Debug, Serialize)]
pub enum GitError {
    #[error("{0}")]
    ProjectOverlayNotFoundError(String),
}