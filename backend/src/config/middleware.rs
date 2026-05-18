use actix_web::error::ErrorForbidden;
use log::error;
use log::info;
use supabase_jwt::Claims;
use supabase_jwt::JwksCache;
use uuid::Uuid;

// Uses a JwksCache constructed once at startup (lives in AppState). Each call
// reuses the shared cache so the JWKS isnt re-fetched on every request.
pub async fn validate_jwt(
    token: Option<String>,
    jwks_cache: &JwksCache,
) -> Result<Uuid, Box<dyn std::error::Error>> {
    if let Some(v) = token {
        match Claims::from_bearer_token(v.as_str(), jwks_cache).await {
            Ok(claims) => {
                info!(
                    "Successfully validated token for user: {}",
                    claims.user_id()
                );
                let uuid = Uuid::parse_str(claims.user_id()).map_err(|e| {
                    error!("Authentication failed: {e}");
                    ErrorForbidden("Authentication failed, try logging in again")
                })?;
                Ok(uuid)
            }
            Err(e) => {
                error!("Authentication failed: {e}");
                Err(ErrorForbidden("Authentication failed").into())
            }
        }
    } else {
        error!("Authentication failed: Missing Token");
        Err(ErrorForbidden("Missing token").into())
    }
}
