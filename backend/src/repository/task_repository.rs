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

pub async fn set_archived(
    db: &SupabaseClient,
    task_id: &Uuid,
    archived: bool,
) -> Result<(), RepoError> {
    db.update("task", task_id.to_string().as_str(), json!({ "archived": archived }))
        .await
        .map_err(|e| {
            error!("Failed updating task archived flag: {e}");
            RepoError::UpdateError(String::from("Failed updating task archived flag"))
        })?;
    Ok(())
}

pub async fn set_kanban_column(
    db: &SupabaseClient,
    task_id: &Uuid,
    column: &str,
) -> Result<(), RepoError> {
    db.update(
        "task",
        task_id.to_string().as_str(),
        json!({ "kanban_column": column }),
    )
    .await
    .map_err(|e| {
        error!("Failed updating task kanban_column: {e}");
        RepoError::UpdateError(String::from("Failed updating task column"))
    })?;
    Ok(())
}

/// Resolve which project a task belongs to. Used to scope permission checks
/// when the caller only has the task id (e.g. the archive endpoint).
pub async fn project_id_of_task(
    db: &SupabaseClient,
    task_id: &Uuid,
) -> Result<Option<Uuid>, RepoError> {
    let rows = db
        .select("task")
        .eq("id", task_id.to_string().as_str())
        .columns(vec!["project_id"])
        .execute()
        .await
        .map_err(|e| {
            error!("Failed locating task project: {e}");
            RepoError::ExtractionError(String::from("Failed locating task project"))
        })?;

    let Some(row) = rows.first() else {
        return Ok(None);
    };
    let Some(pid) = row.get("project_id").and_then(|v| v.as_str()) else {
        return Ok(None);
    };
    Ok(Uuid::parse_str(pid).ok())
}