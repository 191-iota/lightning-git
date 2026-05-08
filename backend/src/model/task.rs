use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::task_type::TaskType;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TaskRes {
    pub id: Uuid,
    pub name: String,
    pub branch_name: String,
    pub task_type: TaskType,
}
