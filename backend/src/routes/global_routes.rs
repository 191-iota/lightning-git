use actix_web::web;

use crate::handler::config_handler::health_check;

use crate::handler::project_handler::create_project;
use crate::handler::project_handler::get_project_file;

use crate::handler::overlay_handler::create_active_overlay;
use crate::handler::overlay_handler::get_overlay;

// Routes starting with "/api" (auth middleware will be added in a later checkpoint)
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/projects", web::post().to(create_project))
            .route(
                "/projects/{id}/{branch}/file",
                web::post().to(get_project_file),
            )
            .route(
                "/overlay/{project_id}/{user_id}/{file_name}",
                web::get().to(get_overlay),
            )
            .route(
                "/overlay/{project_id}/{user_id}/{file_name}/{branch}",
                web::put().to(create_active_overlay),
            ),
    )
    .service(web::scope("").route("/health", web::get().to(health_check)));
}
