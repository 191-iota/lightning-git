use actix_web::body::MessageBody;
use actix_web::error::ErrorUnauthorized;
use actix_web::web;

use crate::config::middleware;
use crate::handler::config_handler::health_check;
use crate::handler::merge_handler::get_merge_conflicts;
use crate::handler::overlay_handler::create_active_overlay;
use crate::handler::overlay_handler::get_overlay;
use crate::handler::overlay_ws::ws_overlay_stream;
use crate::handler::project_handler::create_project;
use crate::handler::project_handler::delete_project;
use crate::handler::project_handler::get_project;
use crate::handler::project_handler::get_project_file;
use crate::handler::project_handler::get_project_members;
use crate::handler::project_handler::update_project;
use crate::handler::task_handler::get_project_tasks;
use crate::handler::task_handler::get_task;
use crate::handler::user_handler::get_user_id_by_username;
use crate::handler::user_handler::github_auth;
use crate::handler::user_handler::github_callback;
use crate::handler::user_handler::login;
use crate::handler::user_handler::register;
use crate::model::user::MiddlewareData;
use actix_web::Error;
use actix_web::HttpMessage;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::middleware::from_fn;
use log::error;

// Routes starting with "/api", which also are protected by the middleware
pub fn init_api_scope(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .wrap(from_fn(auth_filter))
            .route("/tasks/project/{proj_id}", web::get().to(get_project_tasks))
            .route("/tasks/{id}/{proj_id}", web::get().to(get_task))
            .route("/projects", web::post().to(create_project))
            .route("/projects/{id}", web::put().to(update_project))
            .route("/projects/{id}", web::delete().to(delete_project))
            .route("/projects/{id}", web::get().to(get_project))
            .route("/projects/{id}/file", web::post().to(get_project_file))
            .route("/projects/{id}/members", web::get().to(get_project_members))
            .route(
                "/overlay/ws/{project_id}/{user_id}/{file_name:.*}",
                web::get().to(ws_overlay_stream),
            )
            .route(
                "/overlay/{project_id}/{user_id}/{file_name}",
                web::get().to(get_overlay),
            )
            .route(
                "/overlay/{project_id}/{user_id}/{file_name}/{branch}",
                web::put().to(create_active_overlay),
            )
            .route(
                "/merge/{project_id}/{file_name:.*}",
                web::get().to(get_merge_conflicts),
            )
            .route("/user/{username}", web::get().to(get_user_id_by_username)),
    );
}

// Unprotected routes
pub fn init_anon_scope(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .route("/auth/github/callback", web::get().to(github_callback))
            .route("/auth/github/{user_id}", web::get().to(github_auth))
            .route("/health", web::get().to(health_check))
            .route("/register", web::post().to(register))
            // TODO: implement a HTTP only cookie
            .route("/login", web::post().to(login)),
    );
}
async fn auth_filter(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let maybe_token = req
        .headers()
        .get("Authorization")
        .map(|x| x.to_str().unwrap().to_string())
        .or_else(|| {
            req.query_string()
                .split('&')
                .find_map(|p| p.strip_prefix("token="))
                .map(|s| format!("Bearer {}", s))
        });

    match middleware::validate_jwt(maybe_token).await {
        Ok(user_id) => {
            // Insert the user_id from the JWT-Token into ReqData
            req.extensions_mut().insert(MiddlewareData { user_id });
            next.call(req).await
        }
        Err(e) => {
            error!("Authorization failed: {e}");
            Err(ErrorUnauthorized("Authorization failed."))
        }
    }
}
