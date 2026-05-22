use std::str::FromStr;

use log::error;
use serde_json::Value;
use serde_json::json;
use supabase_rs::SupabaseClient;
use uuid::Uuid;

use crate::error::custom_errors::RepoError;
use crate::model::org::{MyOrgRes, OrgMemberRes, OrgRes, OrgRole};

/// Insert a new organization and add the creator as its owner.
pub async fn save_org(
    db: &SupabaseClient,
    id: &Uuid,
    name: &str,
    owner_id: &Uuid,
) -> Result<String, RepoError> {
    let inserted = db
        .insert(
            "organization",
            json!({
                "id": id.to_string(),
                "name": name,
            }),
        )
        .await
        .map_err(|e| {
            error!("Failed inserting org: {e}");
            RepoError::InsertionError(String::from("Failed inserting org"))
        })?;

    add_org_member(db, id, owner_id, OrgRole::Owner).await?;
    Ok(inserted)
}

pub async fn update_org(db: &SupabaseClient, id: &Uuid, name: &str) -> Result<(), RepoError> {
    db.update(
        "organization",
        id.to_string().as_str(),
        json!({ "name": name }),
    )
    .await
    .map_err(|e| {
        error!("Failed updating org: {e}");
        RepoError::UpdateError(String::from("Failed updating org"))
    })?;
    Ok(())
}

pub async fn delete_org(db: &SupabaseClient, id: &Uuid) -> Result<(), RepoError> {
    db.delete("organization", id.to_string().as_str())
        .await
        .map_err(|e| {
            error!("Failed deleting org: {e}");
            RepoError::DeletionError(String::from("Failed deleting org"))
        })
}

pub async fn find_org_by_id(db: &SupabaseClient, id: &Uuid) -> Result<OrgRes, RepoError> {
    let rows = db
        .select("organization")
        .eq("id", id.to_string().as_str())
        .columns(vec!["id", "name"])
        .limit(1)
        .execute()
        .await
        .map_err(|e| {
            error!("Failed retrieving org: {e}");
            RepoError::ExtractionError(String::from("Failed retrieving org"))
        })?;

    let row = rows
        .into_iter()
        .next()
        .ok_or_else(|| RepoError::NotFound(format!("Org {} not found", id)))?;

    serde_json::from_value(row).map_err(|e| RepoError::ExtractionError(e.to_string()))
}

pub async fn add_org_member(
    db: &SupabaseClient,
    org_id: &Uuid,
    user_id: &Uuid,
    role: OrgRole,
) -> Result<String, RepoError> {
    db.insert(
        "organization_members",
        json!({
            "org_id": org_id.to_string(),
            "user_id": user_id.to_string(),
            "role": role.to_string(),
        }),
    )
    .await
    .map_err(|e| {
        error!("Failed adding user to org: {e}");
        RepoError::InsertionError(String::from("Failed adding user to org"))
    })
}

/// Locate the membership row by (org_id, user_id) and delete by row id.
/// Two-step because supabase_rs doesn't accept compound predicates on delete.
pub async fn remove_org_member(
    db: &SupabaseClient,
    org_id: &Uuid,
    user_id: &Uuid,
) -> Result<(), RepoError> {
    // Look up the membership row's synthetic id, then delete by it.
    let rows = db
        .select("organization_members")
        .eq("org_id", org_id.to_string().as_str())
        .eq("user_id", user_id.to_string().as_str())
        .columns(vec!["id"])
        .limit(1)
        .execute()
        .await
        .map_err(|e| {
            error!("Failed locating org member for removal: {e}");
            RepoError::ExtractionError(String::from("Failed locating org member"))
        })?;

    let membership_id = rows
        .first()
        .and_then(|r| r.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| RepoError::NotFound(String::from("Org membership not found")))?
        .to_string();

    db.delete("organization_members", membership_id.as_str())
        .await
        .map_err(|e| {
            error!("Failed removing org member: {e}");
            RepoError::DeletionError(String::from("Failed removing org member"))
        })
}

/// Organizations the user belongs to, each annotated with their role.
/// Two queries: membership rows plus org metadata, joined in memory.
pub async fn list_user_orgs(
    db: &SupabaseClient,
    user_id: &Uuid,
) -> Result<Vec<MyOrgRes>, RepoError> {
    let member_rows = db
        .select("organization_members")
        .eq("user_id", user_id.to_string().as_str())
        .columns(vec!["org_id", "role"])
        .execute()
        .await
        .map_err(|e| {
            error!("Failed listing user orgs: {e}");
            RepoError::ExtractionError(String::from("Failed listing user orgs"))
        })?;

    if member_rows.is_empty() {
        return Ok(vec![]);
    }

    // Build a (org_id -> role) lookup so we can attach the user's role to each org
    let mut role_by_org: std::collections::HashMap<String, OrgRole> =
        std::collections::HashMap::new();
    for r in &member_rows {
        let org_id = r.get("org_id").and_then(|v| v.as_str()).unwrap_or("");
        let role_str = r.get("role").and_then(|v| v.as_str()).unwrap_or("member");
        let role = OrgRole::from_str(role_str).unwrap_or(OrgRole::Member);
        role_by_org.insert(org_id.to_string(), role);
    }

    let org_ids: Vec<&str> = role_by_org.keys().map(|s| s.as_str()).collect();

    let orgs = db
        .select("organization")
        .in_("id", &org_ids)
        .columns(vec!["id", "name"])
        .execute()
        .await
        .map_err(|e| RepoError::ExtractionError(e.to_string()))?;

    let mut out: Vec<MyOrgRes> = Vec::with_capacity(orgs.len());
    for row in orgs {
        let Ok(base) = serde_json::from_value::<OrgRes>(row) else { continue };
        let role = role_by_org.get(&base.id.to_string()).copied().unwrap_or(OrgRole::Member);
        out.push(MyOrgRes { id: base.id, name: base.name, role });
    }
    Ok(out)
}

/// Members of the organization joined with their display names from profiles.
pub async fn list_org_members(
    db: &SupabaseClient,
    org_id: &Uuid,
) -> Result<Vec<OrgMemberRes>, RepoError> {
    let rows = db
        .select("organization_members")
        .eq("org_id", org_id.to_string().as_str())
        .columns(vec!["user_id", "role"])
        .execute()
        .await
        .map_err(|e| {
            error!("Failed listing org members: {e}");
            RepoError::ExtractionError(String::from("Failed listing org members"))
        })?;

    if rows.is_empty() {
        return Ok(vec![]);
    }

    let member_ids: Vec<String> = rows
        .iter()
        .filter_map(|r| r.get("user_id").and_then(|v| v.as_str()).map(String::from))
        .collect();

    let profiles = db
        .select("profiles")
        .in_(
            "id",
            &member_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        )
        .columns(vec!["id", "display_name"])
        .execute()
        .await
        .map_err(|e| RepoError::ExtractionError(e.to_string()))?;

    let mut out: Vec<OrgMemberRes> = Vec::with_capacity(rows.len());
    for row in rows {
        let user_id_str = row.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
        let role_str = row.get("role").and_then(|v| v.as_str()).unwrap_or("member");
        let display_name = profiles
            .iter()
            .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(user_id_str))
            .and_then(|p| p.get("display_name").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();

        let user_id = match Uuid::parse_str(user_id_str) {
            Ok(u) => u,
            Err(_) => continue,
        };
        let role = OrgRole::from_str(role_str).unwrap_or(OrgRole::Member);

        out.push(OrgMemberRes {
            user_id,
            display_name,
            role,
        });
    }
    Ok(out)
}

/// Lists projects belonging to an org. If `is_owner`, returns every project;
/// otherwise only those the requesting user is a project_member of (matching
/// the "org members see only their own projects" rule).
pub async fn list_org_projects(
    db: &SupabaseClient,
    org_id: &Uuid,
    requesting_user: &Uuid,
    is_owner: bool,
) -> Result<Vec<Value>, RepoError> {
    let all = db
        .select("project")
        .eq("org_id", org_id.to_string().as_str())
        .execute()
        .await
        .map_err(|e| {
            error!("Failed listing org projects: {e}");
            RepoError::ExtractionError(String::from("Failed listing org projects"))
        })?;

    if is_owner {
        return Ok(all);
    }

    let member_rows = db
        .select("project_members")
        .eq("user_id", requesting_user.to_string().as_str())
        .columns(vec!["project_id"])
        .execute()
        .await
        .map_err(|e| RepoError::ExtractionError(e.to_string()))?;

    let visible_ids: std::collections::HashSet<String> = member_rows
        .iter()
        .filter_map(|r| {
            r.get("project_id")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .collect();

    Ok(all
        .into_iter()
        .filter(|p| {
            p.get("id")
                .and_then(|v| v.as_str())
                .map(|id| visible_ids.contains(id))
                .unwrap_or(false)
        })
        .collect())
}
