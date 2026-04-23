use actix_web::HttpResponse;

#[utoipa::path(get, path = "/health", tag = "config")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
