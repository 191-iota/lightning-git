use actix_web::HttpResponse;
use actix_web::body::MessageBody;
use actix_web::web;

use crate::config::middleware;
use crate::handler::config_handler::health_check;
use crate::model::app_state::AppState;
use crate::handler::merge_handler::get_merge_conflicts;
use crate::handler::org_handler::add_org_member;
use crate::handler::org_handler::create_org;
use crate::handler::org_handler::delete_org;
use crate::handler::org_handler::get_org;
use crate::handler::org_handler::list_my_orgs;
use crate::handler::org_handler::list_org_members;
use crate::handler::org_handler::list_org_projects;
use crate::handler::org_handler::remove_org_member;
use crate::handler::org_handler::transfer_org_ownership;
use crate::handler::org_handler::update_org;
use crate::handler::overlay_handler::create_active_overlay;
use crate::handler::overlay_handler::get_overlay;
use crate::handler::overlay_handler::wipe_my_overlay;
use crate::handler::overlay_ws::ws_overlay_stream;
use crate::handler::overlay_ws::ws_project_activity;
use crate::handler::project_handler::add_project_member;
use crate::handler::project_handler::create_project;
use crate::handler::project_handler::delete_project;
use crate::handler::project_handler::remove_project_member;
use crate::handler::project_handler::get_project;
use crate::handler::project_handler::get_project_activity;
use crate::handler::project_handler::get_project_file;
use crate::handler::project_handler::get_project_members;
use crate::handler::project_handler::list_project_branches;
use crate::handler::project_handler::list_project_tree;
use crate::handler::project_handler::update_project;
use crate::handler::task_handler::get_project_tasks;
use crate::handler::task_handler::get_task;
use crate::handler::task_handler::set_task_archived;
use crate::handler::task_handler::set_task_column;
use crate::handler::user_handler::get_user_id_by_username;
use crate::handler::user_handler::github_auth;
use crate::handler::user_handler::github_callback;
use crate::handler::user_handler::login;
use crate::handler::user_handler::refresh_token;
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
            .route("/tasks/{task_id}/archive", web::patch().to(set_task_archived))
            .route("/tasks/{task_id}/column", web::patch().to(set_task_column))
            .route("/projects", web::post().to(create_project))
            .route("/projects/{id}", web::put().to(update_project))
            .route("/projects/{id}", web::delete().to(delete_project))
            .route("/projects/{id}", web::get().to(get_project))
            .route("/projects/{id}/file", web::get().to(get_project_file))
            .route("/projects/{id}/tree", web::get().to(list_project_tree))
            .route("/projects/{id}/branches", web::get().to(list_project_branches))
            .route("/projects/{id}/activity", web::get().to(get_project_activity))
            .route("/projects/{id}/activity/ws", web::get().to(ws_project_activity))
            .route("/projects/{id}/members", web::get().to(get_project_members))
            .route("/projects/{id}/members", web::post().to(add_project_member))
            .route(
                "/projects/{id}/members/{user_id}",
                web::delete().to(remove_project_member),
            )
            .route(
                "/overlay/ws/{project_id}/{user_id}/{file_name:.*}",
                web::get().to(ws_overlay_stream),
            )
            .route(
                "/overlay/{project_id}/{user_id}/{file_name:.*}",
                web::get().to(get_overlay),
            )
            // branch is a query param because both branch and file path contain slashes
            .route(
                "/overlay/{project_id}/{user_id}/{file_name:.*}",
                web::put().to(create_active_overlay),
            )
            .route("/overlay/me/{proj_id}", web::delete().to(wipe_my_overlay))
            .route(
                "/merge/{project_id}/{file_name:.*}",
                web::get().to(get_merge_conflicts),
            )
            .route("/user/{username}", web::get().to(get_user_id_by_username))
            .route("/orgs", web::post().to(create_org))
            // must come before /orgs/{id} so "mine" doesnt match as a uuid
            .route("/orgs/mine", web::get().to(list_my_orgs))
            .route("/orgs/{id}", web::get().to(get_org))
            .route("/orgs/{id}", web::put().to(update_org))
            .route("/orgs/{id}", web::delete().to(delete_org))
            .route("/orgs/{id}/members", web::get().to(list_org_members))
            .route("/orgs/{id}/members", web::post().to(add_org_member))
            .route(
                "/orgs/{id}/members/{user_id}",
                web::delete().to(remove_org_member),
            )
            .route("/orgs/{id}/projects", web::get().to(list_org_projects))
            .route("/orgs/{id}/transfer", web::post().to(transfer_org_ownership)),
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
            .route("/login", web::post().to(login))
            .route("/refresh", web::post().to(refresh_token)),
    );
}
// Returns Ok with a 401 ServiceResponse rather than Err so the response flows
// back through the outer Cors wrap and picks up Access-Control-Allow-Origin.
async fn auth_filter(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
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

    // pull the shared JwksCache out of AppState so we dont allocate a new one per request
    let jwks_cache = req
        .app_data::<actix_web::web::Data<AppState>>()
        .expect("AppState missing")
        .jwks_cache
        .clone();

    match middleware::validate_jwt(maybe_token, &jwks_cache).await {
        Ok(user_id) => {
            req.extensions_mut().insert(MiddlewareData { user_id });
            let res = next.call(req).await?;
            Ok(res.map_into_left_body())
        }
        Err(e) => {
            error!("Authorization failed: {e}");
            let (req, _) = req.into_parts();
            let response = HttpResponse::Unauthorized()
                .body("Authorization failed.")
                .map_into_right_body();
            Ok(ServiceResponse::new(req, response))
        }
    }
}
