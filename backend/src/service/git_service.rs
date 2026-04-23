//! This module executes git commands as sub-processes using the tokio runtime

use std::collections::HashSet;
use std::path::Path;
use tokio::process::Command;

use crate::error::custom_errors::GitError;
use crate::error::custom_errors::LGitIoError;

/// Fetches all remote branches and prunes deleted refs
pub async fn fetch(path: &Path) -> Result<(), LGitIoError> {
    handle_git_subprocess(path, &["fetch", "--prune"]).await?;
    Ok(())
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

/// Reads file content from origin/main ref
pub async fn read_file(path: &Path, branch: &str, file_path: &Path) -> Result<String, LGitIoError> {
    fetch(path).await?;
    // CHANGE LATER to origin/current-branch
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
