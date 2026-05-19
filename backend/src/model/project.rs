use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;

use super::task::TaskRes;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectRole {
    Admin,
    Member,
}

impl Display for ProjectRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectRole::Admin => write!(f, "admin"),
            ProjectRole::Member => write!(f, "member"),
        }
    }
}

impl FromStr for ProjectRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(ProjectRole::Admin),
            "member" => Ok(ProjectRole::Member),
            other => Err(format!("Unknown project role: {other}")),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProjectRes {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub name: String,
    pub tasks: Vec<TaskRes>,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct UpdateProjectReq {
    #[validate(length(min = 3, max = 255))]
    pub name: String,
    pub user_ids: Vec<Uuid>,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateProjectReq {
    #[validate(url)]
    pub repo_url: String,
    pub name: String,
    pub create_tasks_retroactively: bool,
    #[schema(value_type = String)]
    pub org_id: Uuid,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProjectMemberRes {
    pub id: Uuid,
    pub display_name: String,
    pub role: ProjectRole,
}

#[derive(Serialize, ToSchema)]
pub struct CreateProjectRes {
    pub proj_id: Uuid,
}
