use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OrgRole {
    Owner,
    Member,
}

impl Display for OrgRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrgRole::Owner => write!(f, "owner"),
            OrgRole::Member => write!(f, "member"),
        }
    }
}

impl FromStr for OrgRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "owner" => Ok(OrgRole::Owner),
            "member" => Ok(OrgRole::Member),
            other => Err(format!("Unknown org role: {other}")),
        }
    }
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateOrgReq {
    #[validate(length(min = 2, max = 64))]
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct CreateOrgRes {
    #[schema(value_type = String)]
    pub org_id: Uuid,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct UpdateOrgReq {
    #[validate(length(min = 2, max = 64))]
    pub name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct OrgRes {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub name: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct AddOrgMemberReq {
    #[schema(value_type = String)]
    pub user_id: Uuid,
    pub role: OrgRole,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct OrgMemberRes {
    #[schema(value_type = String)]
    pub user_id: Uuid,
    pub display_name: String,
    pub role: OrgRole,
}
