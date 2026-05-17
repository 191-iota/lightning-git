//! This module contains functionality for performing CRUD operations on tasks

use log::error;
use serde_json::Value;
use serde_json::json;
use supabase_rs::SupabaseClient;
use uuid::Uuid;

use crate::error::custom_errors::RepoError;
use crate::model::task_type::TaskType;

pub async fn save_task(
    db: &SupabaseClient,
    name: String,
    branch_name: String,
    task_type: TaskType,
    id: String,
    project_id: &Uuid,
) -> Result<String, RepoError> {
    let db_result = db
        .upsert(
            "task",
            id.as_str(),
            json!({
                "name": name,
                "branch_name": branch_name,
                "task_type": task_type,
                "project_id": project_id.to_string(),
            }),
        )
        .await;

    let handled_result = db_result.map_err(|e| {
        error!("Failed inserting task: {e}");
        RepoError::InsertionError(String::from("Failed inserting task"))
    })?;

    Ok(handled_result)
}

// Unused for now
pub async fn delete_task(client: &SupabaseClient, id: String) -> Result<(), RepoError> {
    client.delete("task", id.as_str()).await.map_err(|e| {
        error!("Failed deleting task: {e}");
        RepoError::DeletionError(String::from("Failed deleting Task"))
    })
}

pub async fn find_by_id(client: &SupabaseClient, id: String) -> Result<Value, RepoError> {
    let matches = client
        .select("task")
        .eq("id", id.as_str())
        .limit(1)
        .execute()
        .await;

    let handled_result = matches.map_err(|e| {
        error!("Failed retrieving tasks {e}");
        RepoError::ExtractionError(String::from("Failed retrieving task"))
    })?;

    Ok(handled_result[0].clone())
}

pub async fn find_by_proj(client: &SupabaseClient, id: String) -> Result<Vec<Value>, RepoError> {
    let matches = client
        .select("task")
        .eq("project_id", id.as_str())
        .execute()
        .await;

    let handled_result = matches.map_err(|e| {
        error!("Failed retrieving task {e}");
        RepoError::ExtractionError(String::from("Failed retrieving task"))
    })?;

    Ok(handled_result)
}