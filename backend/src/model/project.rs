use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use validator::Validate;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProjectRes {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub name: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateProjectReq {
    #[validate(url)]
    pub repo_url: String,
    pub name: String,
}

#[derive(Deserialize, ToSchema)]
pub struct FileReadReq {
    pub file_path: String,
}

#[derive(Serialize, ToSchema)]
pub struct CreateProjectRes {
    pub proj_id: Uuid,
}
