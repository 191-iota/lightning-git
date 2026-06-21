use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;
use validator::ValidationError;

fn validate_username(username: &str) -> Result<(), ValidationError> {
    static USERNAME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap());
    if USERNAME_RE.is_match(username) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid username"))
    }
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct LoginPayload {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 64))]
    pub password: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct RegisterPayload {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 3, max = 32))]
    #[validate(custom(function = "validate_username"))]
    pub username: String,
    #[validate(length(min = 8, max = 64))]
    pub password: String,
}

#[derive(Clone)]
pub struct MiddlewareData {
    pub user_id: Uuid,
}

#[derive(Clone, Serialize, Validate)]
pub struct LoginRes {
    pub user_id: Uuid,
    #[validate(email)]
    pub email: String,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct RefreshReq {
    pub refresh_token: String,
}

#[derive(Serialize, ToSchema)]
pub struct RefreshRes {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct GithubCallbackQuery {
    pub code: String,
    pub state: String, // user_id passed through OAuth state param
}

#[derive(Serialize, Validate, Deserialize, ToSchema)]
pub struct UserSearchEntryRes {
    pub display_name: String,
    pub id: Uuid,
}

#[derive(Deserialize)]
pub struct GithubTokenResponse {
    pub access_token: String,
}