use log::error;
use supabase_rs::SupabaseClient;
use uuid::Uuid;

use crate::error::custom_errors::RepoError;

// For "/api..." endpoints, user should be member of the specified project
pub async fn check_repo_permission(
    sb_client: &SupabaseClient,
    project_id: &Uuid,
    user_id: &Uuid,
) -> Result<bool, RepoError> {
    let db_result = sb_client
        .select("project_members")
        .eq("project_id", project_id.to_string().as_str())
        .eq("user_id", user_id.to_string().as_str())
        .execute()
        .await;

    let handled_result = db_result.map_err(|e| {
        error!("Failed checking permissions {e}");
        RepoError::ExtractionError(String::from("Failed checking permissions"))
    })?;

    Ok(handled_result.len() > 0)
}