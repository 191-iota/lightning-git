use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;
use supabase_rs::SupabaseClient;
use uuid::Uuid;

use regex::Regex;

use crate::error::custom_errors::LGitIoError;
use crate::model::task_type::TaskType;
use crate::repository::task_repository;

use super::git_service;

/// Splice an OAuth access token into an https GitHub URL so git clone can
/// reach private repos without prompting for credentials.
pub fn inject_token(repo_url: &str, token: &str) -> String {
    repo_url.replacen("https://", &format!("https://x-access-token:{token}@"), 1)
}

/// Derive Kanban tasks from the project's remote branches.
/// Skips main, master, HEAD, and any branch that already has a task row.
pub async fn detect_and_create_tasks(
    repo_path: &Path,
    db: &SupabaseClient,
    project_id: &Uuid,
) -> Result<(), LGitIoError> {
    let branches = git_service::list_remote_branches(repo_path).await?;
    let re = Regex::new(r"[/_\-.]").unwrap();

    // skip branches that already have a task row for this project
    let existing = task_repository::find_by_proj(db, project_id.to_string()).await?;
    let existing_branches: HashSet<String> = existing
        .iter()
        .filter_map(|v| v.get("branch_name").and_then(|n| n.as_str()).map(String::from))
        .collect();

    for b in branches.into_iter() {
        // main/master/HEAD are integration targets, not work units — never tasks
        if matches!(b.as_str(), "main" | "master" | "HEAD") {
            continue;
        }
        if existing_branches.contains(&b) {
            continue;
        }

        let parts: Vec<&str> = re.split(b.as_str()).collect();

        let branch_type = match parts[0].parse::<u32>() {
            Ok(_) => match parts.get(1).map(|s| s.parse::<u32>()) {
                Some(Ok(_)) => "Unknown",
                Some(Err(_)) => parts[1],
                None => "Unknown",
            },
            Err(_) => parts[0],
        };

        let task_name = b
            .strip_prefix(branch_type)
            .unwrap_or(&b)
            .trim_start_matches(|c: char| c == '/' || c == '-' || c == '_' || c == '.')
            .to_string();

        task_repository::save_task(
            db,
            task_name,
            b.clone(),
            TaskType::from_str(branch_type).unwrap(),
            Uuid::new_v4().to_string(),
            project_id,
        )
        .await?;
    }

    Ok(())
}
