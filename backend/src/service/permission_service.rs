use log::error;
use supabase_rs::SupabaseClient;
use uuid::Uuid;

use crate::error::custom_errors::RepoError;
use crate::model::app_state::AppState;
use crate::model::org::OrgRole;
use crate::model::project::ProjectRole;

/// Returns true if user_id is org-owner of the org that owns project_id.
/// Returns false if the project doesn't exist, the user isn't in the org, or the user's role isn't owner.
async fn is_org_owner_of_project(
    sb_client: &SupabaseClient,
    project_id: &Uuid,
    user_id: &Uuid,
) -> Result<bool, RepoError> {
    let project_rows = sb_client
        .select("project")
        .eq("id", project_id.to_string().as_str())
        .columns(vec!["org_id"])
        .limit(1)
        .execute()
        .await
        .map_err(|e| {
            error!("Failed fetching project for permission check: {e}");
            RepoError::ExtractionError(String::from("Failed checking permissions"))
        })?;

    let Some(org_id) = project_rows
        .first()
        .and_then(|row| row.get("org_id"))
        .and_then(|v| v.as_str())
    else {
        return Ok(false);
    };

    check_org_role(sb_client, org_id, &user_id.to_string(), OrgRole::Owner).await
}

async fn check_org_role(
    sb_client: &SupabaseClient,
    org_id: &str,
    user_id: &str,
    required_role: OrgRole,
) -> Result<bool, RepoError> {
    let rows = sb_client
        .select("organization_members")
        .eq("org_id", org_id)
        .eq("user_id", user_id)
        .columns(vec!["role"])
        .limit(1)
        .execute()
        .await
        .map_err(|e| {
            error!("Failed checking org membership: {e}");
            RepoError::ExtractionError(String::from("Failed checking permissions"))
        })?;

    let Some(role_str) = rows
        .first()
        .and_then(|row| row.get("role"))
        .and_then(|v| v.as_str())
    else {
        return Ok(false);
    };

    Ok(role_satisfies_org(role_str, required_role))
}

fn role_satisfies_org(actual: &str, required: OrgRole) -> bool {
    match required {
        OrgRole::Member => actual == "owner" || actual == "member",
        OrgRole::Owner => actual == "owner",
    }
}

fn role_satisfies_project(actual: &str, required: ProjectRole) -> bool {
    match required {
        ProjectRole::Member => actual == "admin" || actual == "member",
        ProjectRole::Admin => actual == "admin",
    }
}

pub async fn check_project_permission(
    state: &AppState,
    project_id: &Uuid,
    user_id: &Uuid,
    required_role: ProjectRole,
) -> Result<bool, RepoError> {
    if is_org_owner_of_project(&state.sb_client, project_id, user_id).await? {
        return Ok(true);
    }

    let rows = state
        .sb_client
        .select("project_members")
        .eq("project_id", project_id.to_string().as_str())
        .eq("user_id", user_id.to_string().as_str())
        .columns(vec!["role"])
        .limit(1)
        .execute()
        .await
        .map_err(|e| {
            error!("Failed checking project membership: {e}");
            RepoError::ExtractionError(String::from("Failed checking permissions"))
        })?;

    let Some(role_str) = rows
        .first()
        .and_then(|row| row.get("role"))
        .and_then(|v| v.as_str())
    else {
        return Ok(false);
    };

    Ok(role_satisfies_project(role_str, required_role))
}

pub async fn check_org_permission(
    state: &AppState,
    org_id: &Uuid,
    user_id: &Uuid,
    required_role: OrgRole,
) -> Result<bool, RepoError> {
    check_org_role(
        &state.sb_client,
        org_id.to_string().as_str(),
        user_id.to_string().as_str(),
        required_role,
    )
    .await
}
