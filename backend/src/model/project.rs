use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::task::TaskRes;
use validator::Validate;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProjectRes {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub name: String,
    pub tasks: Vec<TaskRes>,
}

#[derive(Deserialize, ToSchema)]
pub struct DeleteProjectReq {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub repo_url: String,
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
}

#[derive(Deserialize, ToSchema)]
pub struct FileReadReq {
    pub file_path: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProjectMemberRes {
    pub id: Uuid,
    pub display_name: String,
}

#[derive(Serialize, ToSchema)]
pub struct CreateProjectRes {
    pub proj_id: Uuid,
}
