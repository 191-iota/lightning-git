//! This module executes git commands as sub-processes using the tokio runtime

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::time::Instant;
use tokio::process::Command;

use dashmap::DashMap;
use once_cell::sync::Lazy;

use crate::error::custom_errors::GitError;
use crate::error::custom_errors::LGitIoError;

// throttle so multiple reads within the same window dont each pay the
// network cost of "git fetch" on the same repo. session start used to fire
// ~N+2 fetches per click; with the throttle those collapse to one.
const FETCH_TTL: Duration = Duration::from_secs(30);
static LAST_FETCH: Lazy<DashMap<PathBuf, Instant>> = Lazy::new(DashMap::new);

/// Fetches all remote branches and prunes deleted refs. Always hits the network.
pub async fn fetch(path: &Path) -> Result<(), LGitIoError> {
    handle_git_subprocess(path, &["fetch", "--prune"]).await?;
    LAST_FETCH.insert(path.to_path_buf(), Instant::now());
    Ok(())
}

/// Like "fetch", but skips when the last successful fetch is within FETCH_TTL.
/// Use this from request-driven code paths so a burst of reads doesnt thrash.
pub async fn maybe_fetch(path: &Path) -> Result<(), LGitIoError> {
    if let Some(t) = LAST_FETCH.get(path) {
        if t.elapsed() < FETCH_TTL {
            return Ok(());
        }
    }
    fetch(path).await
}

/// Clones a repository from source URL to destination path
pub async fn clone_repo(src: &str, dst: &Path) -> Result<(), LGitIoError> {
    let parent = dst.parent().ok_or_else(|| {
        LGitIoError::GitError(GitError::ProjectOverlayNotFoundError(
            "Destination has no parent".to_string(),
        ))
    })?;

    let dst = dst
        .file_name()
        .ok_or_else(|| {
            LGitIoError::GitError(GitError::ProjectOverlayNotFoundError(
                "Invalid dst path".to_string(),
            ))
        })?
        .to_str()
        .ok_or_else(|| {
            LGitIoError::GitError(GitError::ProjectOverlayNotFoundError(
                "Invalid dst name".to_string(),
            ))
        })?;

    handle_git_subprocess(parent, &["clone", src, dst]).await?;
    Ok(())
}

/// Returns all remote branch names excluding HEAD references
pub async fn list_remote_branches(repo_path: &Path) -> Result<HashSet<String>, LGitIoError> {
    let output = handle_git_subprocess(repo_path, &["branch", "-r"]).await?;

    let branches: HashSet<String> = output
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.contains("->"))
        .map(|s| s.strip_prefix("origin/").unwrap_or(s).to_string())
        .collect();

    // Expected result: vec!["origin/feature-a", ...]
    Ok(branches)
}

/// Deletes a repository directory
pub async fn delete_repo(path: &Path) -> Result<(), LGitIoError> {
    tokio::fs::remove_dir_all(path)
        .await
        .map_err(|e| LGitIoError::GitError(GitError::ProjectOverlayNotFoundError(e.to_string())))
}

/// Lists every file path tracked at origin/{branch}
pub async fn list_files(repo_path: &Path, branch: &str) -> Result<Vec<String>, LGitIoError> {
    maybe_fetch(repo_path).await?;
    let spec = format!("origin/{}", branch);
    let output =
        handle_git_subprocess(repo_path, &["ls-tree", "-r", "--name-only", spec.as_str()]).await?;
    Ok(output.lines().map(|s| s.to_string()).collect())
}

/// Reads file content from origin/{branch} ref
pub async fn read_file(path: &Path, branch: &str, file_path: &Path) -> Result<String, LGitIoError> {
    maybe_fetch(path).await?;
    let branch = format!("origin/{}", branch);
    let spec = format!("{}:{}", branch.trim(), file_path.display());
    Ok(handle_git_subprocess(path, &["show", spec.as_str()]).await?)
}

/// Executes a git command and returns stdout as a string
async fn handle_git_subprocess(repo_path: &Path, args: &[&str]) -> Result<String, LGitIoError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| LGitIoError::GitError(GitError::ProjectOverlayNotFoundError(e.to_string())))?;

    if !output.status.success() {
        return Err(LGitIoError::GitError(
            GitError::ProjectOverlayNotFoundError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ),
        ));
    }

    // If successful, return the string
    String::from_utf8(output.stdout).map_err(|e| {
        LGitIoError::GitError(GitError::ProjectOverlayNotFoundError(format!(
            "git output not valid UTF-8: {e}"
        )))
    })
}
