use std::env;

use actix_web::error::ErrorForbidden;
use log::error;
use log::info;
use supabase_jwt::Claims;
use supabase_jwt::JwksCache;
use uuid::Uuid;

// TODO: rename to middleware
// TODO: implement security configurations
// TODO: implement additional securiy checks for operations involving git

// 1. Implement middleware validation
pub async fn validate_jwt(token: Option<String>) -> Result<Uuid, Box<dyn std::error::Error>> {
    // 1. Initialize the JWKS cache with your Supabase URL
    let supabase_url = env::var("SUPABASE_URL").expect("Could not find SPABASE_URL env");
    let jwks_url = format!("{supabase_url}/auth/v1/.well-known/jwks.json");
    let jwks_cache = JwksCache::new(jwks_url.as_str());

    // 3. Validate the JWT and extract claims
    if let Some(v) = token {
        match Claims::from_bearer_token(v.as_str(), &jwks_cache).await {
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