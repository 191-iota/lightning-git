use log::error;
use serde_json::json;
use supabase_rs::SupabaseClient;
use uuid::Uuid;

use crate::error::custom_errors::RepoError;
use crate::model::user::UserSearchEntryRes;

pub async fn update_access_token(
    db: &SupabaseClient,
    user_id: &String,
    access_token: String,
) -> Result<(), RepoError> {
    let db_result = db
        .update(
            "profiles",
            user_id.to_string().as_str(),
            json!({
                "github_token": access_token,
            }),
        )
        .await;

    db_result.map_err(|e| {
        error!("Failed updating project: {e}");
        RepoError::UpdateError(String::from("Failed updating project"))
    })?;

    Ok(())
}

pub async fn get_access_token(
    db: &SupabaseClient,
    user_id: &Uuid,
) -> Result<Option<String>, RepoError> {
    let db_result = db
        .select("profiles")
        .eq("id", user_id.to_string().as_str())
        .columns(vec!["github_token"])
        .execute()
        .await;

    let response = db_result.map_err(|e| {
        error!("Failed fetching project members: {e}");
        RepoError::ExtractionError(String::from("Failed fetching project members"))
    })?;

    let token = response
        .get(0)
        .and_then(|row| row.get("github_token"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    Ok(token)
}

pub async fn get_user_id_by_username(
    db: &SupabaseClient,
    username: &str,
) -> Result<Vec<UserSearchEntryRes>, RepoError> {
    let db_result = db
        .select("profiles")
        .eq("display_name", username)
        .columns(vec!["display_name", "id"])
        .limit(5)
        .execute()
        .await;

    let response = db_result.map_err(|e| {
        error!("Failed fetching user search results: {e}");
        RepoError::ExtractionError(String::from("Failed fetching user search results"))
    })?;

    let entries: Vec<UserSearchEntryRes> = response
        .iter()
        .filter_map(|row| serde_json::from_value(row.clone()).ok())
        .collect();
    Ok(entries)
}