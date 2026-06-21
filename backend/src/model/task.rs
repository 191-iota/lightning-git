use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::task_type::TaskType;

#[derive(Serialize, Deserialize, ToSchema, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KanbanColumn {
    Todo,
    InProgress,
    Review,
    Merged,
}

impl KanbanColumn {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            KanbanColumn::Todo => "todo",
            KanbanColumn::InProgress => "in_progress",
            KanbanColumn::Review => "review",
            KanbanColumn::Merged => "merged",
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TaskRes {
    pub id: Uuid,
    pub name: String,
    pub branch_name: String,
    pub task_type: TaskType,
    #[serde(default)]
    pub archived: bool,
    #[serde(default = "default_kanban_column")]
    pub kanban_column: KanbanColumn,
}

fn default_kanban_column() -> KanbanColumn {
    KanbanColumn::InProgress
}

#[derive(Deserialize, ToSchema)]
pub struct SetArchivedReq {
    pub archived: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct SetColumnReq {
    pub kanban_column: KanbanColumn,
}
