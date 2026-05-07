use actix_cors::Cors;
use actix_web::web;
use dashmap::DashMap;
use std::env;
use std::path::PathBuf;
use utoipa::openapi::security::ApiKey;
use utoipa::openapi::security::ApiKeyValue;
use uuid::Uuid;

use self::model::app_state::AppState;
use self::model::overlay::ProjectLiveState;
use self::routes::global_routes;
use crate::handler::config_handler::__path_health_check;
use crate::handler::merge_handler::__path_get_merge_conflicts;
use crate::handler::overlay_handler::__path_create_active_overlay;
use crate::handler::overlay_handler::__path_get_overlay;
use crate::handler::overlay_ws::__path_ws_overlay_stream;
use crate::handler::project_handler::__path_create_project;
use crate::handler::project_handler::__path_get_project_file;
use crate::model::overlay::Conflict;
use crate::model::overlay::OverlayViewRes;
use crate::model::project::CreateProjectReq;
use crate::model::project::CreateProjectRes;
use crate::model::project::FileReadReq;
use actix_web::App;
use actix_web::HttpServer;
use actix_web::middleware::Logger;
use dotenv::dotenv;
use env_logger::Env;
use log::info;
use log::warn;
use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::SecurityScheme;
use utoipa_swagger_ui::SwaggerUi;
mod error;
mod handler;
mod model;
mod routes;
mod service;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let address = setup_address();

    info!("Running at http://{}:{}", address.0, address.1);

    env_logger::init_from_env(Env::default().default_filter_or("info"));
    let app_state = web::Data::new(init_app_state().await);
    struct SecuritySchemas;

    impl Modify for SecuritySchemas {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            let components = openapi.components.as_mut().unwrap();
            let value = ApiKeyValue::with_description("Authorization", "Bearer ey...");
            let scheme = SecurityScheme::ApiKey(ApiKey::Header(value));
            components.add_security_scheme("Authorization", scheme);
        }
    }

    #[derive(OpenApi)]
    #[openapi(
        paths(
            health_check,
            create_project,
            get_project_file,
            get_overlay,
            create_active_overlay,
            ws_overlay_stream,
            get_merge_conflicts,
        ),
        components(
            schemas(
                CreateProjectReq,
                CreateProjectRes,
                FileReadReq,
                OverlayViewRes,
                Conflict,
            ),
        ),
        security(( "Authorization" = [] )),
        modifiers(&SecuritySchemas),
        tags(
            (name = "project", description = "Project endpoints"),
            (name = "overlay", description = "Overlay endpoints"),
            (name = "merge", description = "Merge endpoints"),
            (name = "config", description = "Config endpoints"),
        )
    )]
    struct ApiDoc;
    let openapi = ApiDoc::openapi();

    HttpServer::new(move || {
        // Actix initializes multiple worker threads, which is why we need to clone elements
        App::new()
            .service(
                SwaggerUi::new("/swagger/{_:.*}").url("/api-doc/openapi.json", openapi.clone()),
            )
            .wrap(Logger::default())
            .wrap(
                Cors::default()
                    .allowed_origin("http://localhost:5173")
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec!["Content-Type", "Authorization"])
                    .max_age(3600),
            )
            .app_data(app_state.clone())
            .configure(global_routes::init_routes)
    })
    .bind(format!("{}:{}", address.0, address.1))?
    .run()
    .await
}

fn setup_address() -> (String, String) {
    let host = env::var("HOST").unwrap_or_else(|_| {
        warn!("Could not find HOST env, defaulting to 127.0.0.1");
        "127.0.0.1".to_string()
    });

    let port = env::var("PORT").unwrap_or_else(|_| {
        warn!("Could not find PORT env, defaulting to 8080");
        "8080".to_string()
    });

    (host, port)
}

async fn init_app_state() -> AppState {
    let repo_location = env::var("GIT_REPO_DEV").expect("Could not find GIT_REPO_DEV");
    let repo_loc_path = PathBuf::from(repo_location);

    // Uncomment when the time comes
    // let repositories_location_prod = env::var("GIT_REPO_DEV").expect("Could not find GIT_REPO_DEV");
    //

    AppState {
        repo_states: DashMap::<Uuid, ProjectLiveState>::new(),
        repo_loc: repo_loc_path,
    }
}
