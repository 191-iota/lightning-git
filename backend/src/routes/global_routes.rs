use actix_web::web;

use crate::handler::config_handler::health_check;
use crate::handler::project_handler::create_project;
use crate::handler::project_handler::get_project_file;

// Routes starting with "/api" (auth middleware will be added in a later checkpoint)
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/projects", web::post().to(create_project))
            .route(
                "/projects/{id}/{branch}/file",
                web::post().to(get_project_file),
            ),
    )
    .service(web::scope("").route("/health", web::get().to(health_check)));
}
