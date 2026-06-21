use std::fmt::Display;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema, Deserialize, Clone, Copy)]
pub enum TaskType {
    Bug,
    Feature,
    Improvement,
    Unknown,
}

impl Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::Bug => write!(f, "Bug"),
            TaskType::Feature => write!(f, "Feature"),
            TaskType::Improvement => write!(f, "Improvement"),
            _ => write!(f, "Unknown"),
        }
    }
}

impl FromStr for TaskType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bug" => Ok(TaskType::Bug),
            "feature" => Ok(TaskType::Feature),
            "improvement" => Ok(TaskType::Improvement),
            _ => Ok(TaskType::Unknown),
        }
    }
}
