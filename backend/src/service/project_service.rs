use std::path::Path;
use std::str::FromStr;
use supabase_rs::SupabaseClient;
use uuid::Uuid;

use regex::Regex;

use crate::error::custom_errors::LGitIoError;
use crate::model::task_type::TaskType;
use crate::repository::task_repository;

use super::git_service;

pub fn inject_token(repo_url: &str, token: &str) -> String {
    repo_url.replacen("https://", &format!("https://x-access-token:{token}@"), 1)
}

pub async fn detect_and_create_tasks(
    repo_path: &Path,
    db: &SupabaseClient,
) -> Result<(), LGitIoError> {
    let branches = git_service::list_remote_branches(repo_path).await?;
    let re = Regex::new(r"[/_\-.]").unwrap();

    for b in branches.into_iter() {
        // Check if current branch name starts with a number or word

        let parts: Vec<&str> = re.split(b.as_str()).collect();

        let branch_type = match parts[0].parse::<u32>() {
            Ok(_) => match parts[1].parse::<u32>() {
                Ok(_) => "Unknown",
                Err(_) => parts[1],
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
        )
        .await?;
    }

    Ok(())
}
