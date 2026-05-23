use actix_web::HttpResponse;
use actix_web::web;
use serde_json::json;

use crate::model::app_state::AppState;

/// Health probe. Verifies the process is up and that Supabase responds to a
/// lightweight select, so deployment environment only keeps the container if both are healthy.
#[utoipa::path(get, path = "/health", tag = "config")]
pub async fn health_check(state: web::Data<AppState>) -> HttpResponse {
    let probe = state
        .sb_client
        .select("organization")
        .columns(vec!["id"])
        .limit(1)
        .execute()
        .await;

    match probe {
        Ok(_) => HttpResponse::Ok().json(json!({ "status": "ok", "supabase": "up" })),
        Err(e) => {
            log::warn!("Health check: Supabase probe failed: {e}");
            HttpResponse::ServiceUnavailable().json(json!({
                "status": "degraded",
                "supabase": "down",
                "error": e.to_string()
            }))
        }
    }
}
