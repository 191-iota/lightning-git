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

// repo errors are currently redundantly handled per method
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
    db.update(
        "project",
        id.to_string().as_str(),
        json!({ "name": update_req.name }),
    )
    .await
    .map_err(|e| {
        error!("Failed updating project: {e}");
        RepoError::UpdateError(String::from("Failed updating project"))
    })?;
    Ok(id.to_string())
}

/// Project members joined with their display names from the profiles table.
/// Two queries: one for membership rows, one for the names in a single in_ filter.
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

/// Look up the project_members join-row id then delete by row id.
/// Two-step because supabase_rs doesn't accept compound predicates on delete.
pub async fn remove_user_from_project(
    db: &SupabaseClient,
    project_id: &Uuid,
    user_id: &Uuid,
) -> Result<(), RepoError> {
    let rows = db
        .select("project_members")
        .eq("project_id", project_id.to_string().as_str())
        .eq("user_id", user_id.to_string().as_str())
        .columns(vec!["id"])
        .execute()
        .await
        .map_err(|e| {
            error!("Failed locating project member row: {e}");
            RepoError::ExtractionError(String::from("Failed locating project member"))
        })?;

    let row_id = rows
        .first()
        .and_then(|r| r.get("id").and_then(|v| v.as_str()))
        .ok_or_else(|| RepoError::ExtractionError(String::from("Member not in project")))?
        .to_string();

    db.delete("project_members", row_id.as_str())
        .await
        .map_err(|e| {
            error!("Failed removing project member: {e}");
            RepoError::DeletionError(String::from("Failed removing project member"))
        })?;

    Ok(())
}

/// How many admins a project has. Used to refuse removing the last admin.
pub async fn count_project_admins(
    db: &SupabaseClient,
    project_id: &Uuid,
) -> Result<usize, RepoError> {
    let rows = db
        .select("project_members")
        .eq("project_id", project_id.to_string().as_str())
        .eq("role", "admin")
        .columns(vec!["user_id"])
        .execute()
        .await
        .map_err(|e| {
            error!("Failed counting project admins: {e}");
            RepoError::ExtractionError(String::from("Failed counting project admins"))
        })?;
    Ok(rows.len())
}

/// The user's role on the project, or None if they are not a member.
pub async fn get_project_member_role(
    db: &SupabaseClient,
    project_id: &Uuid,
    user_id: &Uuid,
) -> Result<Option<ProjectRole>, RepoError> {
    let rows = db
        .select("project_members")
        .eq("project_id", project_id.to_string().as_str())
        .eq("user_id", user_id.to_string().as_str())
        .columns(vec!["role"])
        .execute()
        .await
        .map_err(|e| {
            error!("Failed reading project member role: {e}");
            RepoError::ExtractionError(String::from("Failed reading project member role"))
        })?;

    let Some(row) = rows.first() else { return Ok(None) };
    let role_str = row.get("role").and_then(|v| v.as_str()).unwrap_or("member");
    Ok(Some(ProjectRole::from_str(role_str).unwrap_or(ProjectRole::Member)))
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
