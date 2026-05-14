use std::collections::HashSet;
use std::str::FromStr;

use log::error;
use serde_json::Value;
use serde_json::json;
use supabase_rs::SupabaseClient;
use uuid::Uuid;

use crate::error::custom_errors::RepoError;
use crate::model::project::ProjectMemberRes;
use crate::model::project::ProjectRole;
use crate::model::project::UpdateProjectReq;

// repo errors are currently redundantly handled for extensibility purposes
// for repo methods to scale well, one calc per http call is important -> sql joins, orm,
// responseobjects in one method
pub async fn save_project(
    db: &SupabaseClient,
    id: &Uuid,
    name: &str,
    repo_url: &str,
    org_id: &Uuid,
    creator_id: &Uuid,
) -> Result<String, RepoError> {
    let db_result = db
        .insert(
            "project",
            json!({
                "id": id.to_string(),
                "name": name,
                "repo_url": repo_url,
                "org_id": org_id.to_string(),
            }),
        )
        .await;
    let handled_result = db_result.map_err(|e| {
        error!("Failed inserting project: {e}");
        RepoError::InsertionError(String::from("Failed inserting project"))
    })?;
    add_user_to_project(db, id, creator_id, ProjectRole::Admin).await?;
    Ok(handled_result)
}

pub async fn update_project(
    db: &SupabaseClient,
    id: &Uuid,
    update_req: UpdateProjectReq,
) -> Result<String, RepoError> {
    let db_result = db
        .update(
            "project",
            id.to_string().as_str(),
            json!({
                "name": update_req.name
            }),
        )
        .await;

    db_result.map_err(|e| {
        error!("Failed updating project: {e}");
        RepoError::UpdateError(String::from("Failed updating project"))
    })?;
    let existing_ids: HashSet<String> = get_project_members_id(db, id).await?.into_iter().collect();

    for user_id in update_req.user_ids {
        if !existing_ids.contains(&user_id.to_string()) {
            add_user_to_project(db, id, &user_id, ProjectRole::Member).await?;
        }
    }
    Ok(id.to_string())
}

pub async fn get_project_members_id(
    db: &SupabaseClient,
    project_id: &Uuid,
) -> Result<Vec<String>, RepoError> {
    let db_result = db
        .select("project_members")
        .eq("project_id", project_id.to_string().as_str())
        .columns(vec!["user_id"])
        .execute()
        .await;

    let response = db_result.map_err(|e| {
        error!("Failed fetching project members: {e}");
        RepoError::ExtractionError(String::from("Failed fetching project members"))
    })?;

    Ok(response
        .into_iter()
        .filter_map(|m| m["user_id"].as_str().map(String::from))
        .collect())
}

pub async fn get_project_members_full(
    db: &SupabaseClient,
    project_id: &Uuid,
) -> Result<Vec<ProjectMemberRes>, RepoError> {
    let member_rows = db
        .select("project_members")
        .eq("project_id", project_id.to_string().as_str())
        .columns(vec!["user_id", "role"])
        .execute()
        .await
        .map_err(|e| {
            error!("Failed fetching project members: {e}");
            RepoError::ExtractionError(String::from("Failed fetching project members"))
        })?;

    if member_rows.is_empty() {
        return Ok(vec![]);
    }

    let member_ids: Vec<String> = member_rows
        .iter()
        .filter_map(|m| m.get("user_id").and_then(|v| v.as_str()).map(String::from))
        .collect();

    let users = db
        .select("profiles")
        .in_(
            "id",
            &member_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        )
        .columns(vec!["id", "display_name"])
        .execute()
        .await
        .map_err(|e| RepoError::ExtractionError(e.to_string()))?;

    let mut out: Vec<ProjectMemberRes> = Vec::with_capacity(member_rows.len());
    for m in member_rows {
        let uid_str = m.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
        let role_str = m.get("role").and_then(|v| v.as_str()).unwrap_or("member");
        let id = match Uuid::parse_str(uid_str) {
            Ok(u) => u,
            Err(_) => continue,
        };
        let display_name = users
            .iter()
            .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(uid_str))
            .and_then(|p| p.get("display_name").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        let role = ProjectRole::from_str(role_str).unwrap_or(ProjectRole::Member);

        out.push(ProjectMemberRes {
            id,
            display_name,
            role,
        });
    }
    Ok(out)
}

pub async fn add_user_to_project(
    db: &SupabaseClient,
    id: &Uuid,
    user_id: &Uuid,
    role: ProjectRole,
) -> Result<String, RepoError> {
    let db_result = db
        .insert(
            "project_members",
            json!({
                "project_id": id.to_string().as_str(),
                "user_id": user_id.to_string().as_str(),
                "role": role.to_string(),
            }),
        )
        .await;

    let handled_result = db_result.map_err(|e| {
        error!("Failed adding user to project: {e}");
        RepoError::InsertionError(String::from("Failed adding user to project"))
    })?;

    Ok(handled_result)
}

pub async fn delete_project(client: &SupabaseClient, id: String) -> Result<(), RepoError> {
    client.delete("project", id.as_str()).await.map_err(|e| {
        error!("Failed deleting project: {e}");
        RepoError::DeletionError(String::from("Failed deleting Task"))
    })
}

pub async fn find_project_by_id(client: &SupabaseClient, id: String) -> Result<Value, RepoError> {
    let db_result = client
        .select("project")
        .eq("id", id.as_str())
        .limit(1)
        .execute()
        .await;

    let handled_result = db_result.map_err(|e| {
        error!("Failed retrieving project {e}");
        RepoError::ExtractionError(String::from("Failed retrieving project"))
    })?;

    Ok(handled_result[0].clone())
}

pub async fn find_repo_url_by_id(client: &SupabaseClient, id: String) -> Result<String, RepoError> {
    let db_result = client
        .from("project")
        .columns(vec!["repo_url"])
        .eq("id", id.as_str())
        .limit(1)
        .execute()
        .await;

    let handled_result = db_result.map_err(|e| {
        error!("Failed retrieving repo_url for project {id}: {e}");
        RepoError::ExtractionError(String::from(
            "Could not find repository url for this project",
        ))
    })?;

    let v = handled_result
        .first()
        .and_then(|val| val.get("repo_url"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            error!("repo_url missing or not a string for project {id}");
            RepoError::ExtractionError(String::from(
                "Could not find repository url for this project",
            ))
        })?;
    Ok(v)
}
