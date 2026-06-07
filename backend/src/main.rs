use actix_cors::Cors;
use actix_web::web;
use dashmap::DashMap;
use std::env;
use std::path::PathBuf;
use supabase_auth::models::AuthClient;
use utoipa::openapi::security::ApiKey;
use utoipa::openapi::security::ApiKeyValue;
use uuid::Uuid;

use self::model::app_state::AppState;
use self::model::overlay::ProjectLiveState;
use self::routes::global_routes;
use crate::handler::config_handler::__path_health_check;
use crate::handler::merge_handler::__path_get_merge_conflicts;
use crate::handler::org_handler::__path_add_org_member;
use crate::handler::org_handler::__path_create_org;
use crate::handler::org_handler::__path_delete_org;
use crate::handler::org_handler::__path_get_org;
use crate::handler::org_handler::__path_list_my_orgs;
use crate::handler::org_handler::__path_list_org_members;
use crate::handler::org_handler::__path_list_org_projects;
use crate::handler::org_handler::__path_remove_org_member;
use crate::handler::org_handler::__path_transfer_org_ownership;
use crate::handler::org_handler::__path_update_org;
use crate::handler::overlay_handler::__path_create_active_overlay;
use crate::handler::overlay_handler::__path_get_overlay;
use crate::handler::overlay_handler::__path_wipe_my_overlay;
use crate::handler::overlay_ws::__path_ws_overlay_stream;
use crate::handler::overlay_ws::__path_ws_project_activity;
use crate::handler::project_handler::__path_add_project_member;
use crate::handler::project_handler::__path_create_project;
use crate::handler::project_handler::__path_delete_project;
use crate::handler::project_handler::__path_remove_project_member;
use crate::handler::project_handler::__path_get_project;
use crate::handler::project_handler::__path_get_project_file;
use crate::handler::project_handler::__path_get_project_activity;
use crate::handler::project_handler::__path_get_project_members;
use crate::handler::project_handler::__path_list_project_branches;
use crate::handler::project_handler::__path_list_project_tree;
use crate::handler::project_handler::__path_update_project;
use crate::handler::task_handler::__path_get_project_tasks;
use crate::handler::task_handler::__path_get_task;
use crate::handler::task_handler::__path_set_task_archived;
use crate::handler::task_handler::__path_set_task_column;
use crate::handler::user_handler::__path_get_user_id_by_username;
use crate::handler::user_handler::__path_login;
use crate::handler::user_handler::__path_refresh_token;
use crate::handler::user_handler::__path_register;
use crate::handler::user_handler::__path_update_username;
use crate::model::org::AddOrgMemberReq;
use crate::model::org::CreateOrgReq;
use crate::model::org::CreateOrgRes;
use crate::model::org::MyOrgRes;
use crate::model::org::OrgMemberRes;
use crate::model::org::OrgRes;
use crate::model::org::OrgRole;
use crate::model::org::TransferOrgOwnershipReq;
use crate::model::org::UpdateOrgReq;
use crate::model::overlay::ActiveEdit;
use crate::model::overlay::Comment;
use crate::model::overlay::Conflict;
use crate::model::overlay::OverlayViewRes;
use crate::model::project::AddProjectMemberReq;
use crate::model::project::CreateProjectReq;
use crate::model::project::CreateProjectRes;
use crate::model::project::ProjectMemberRes;
use crate::model::project::ProjectRes;
use crate::model::project::ProjectRole;
use crate::model::project::ProjectTreeRes;
use crate::model::project::UpdateProjectReq;
use crate::model::task::KanbanColumn;
use crate::model::task::SetArchivedReq;
use crate::model::task::SetColumnReq;
use crate::model::task::TaskRes;
use crate::model::user::LoginPayload;
use crate::model::user::RefreshReq;
use crate::model::user::RefreshRes;
use crate::model::user::RegisterPayload;
use crate::model::user::UpdateUsernamePayload;
use crate::model::user::UpdateUsernameRes;
use crate::model::user::UserSearchEntryRes;
use actix_web::App;
use actix_web::HttpServer;
use actix_web::middleware::Logger;
use dotenv::dotenv;
use env_logger::Env;
use log::info;
use log::warn;
use model::task_type::TaskType;
use supabase_rs::SupabaseClient;
use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::SecurityScheme;
use utoipa_swagger_ui::SwaggerUi;
mod error;
mod handler;
mod model;
mod routes;

mod config;
mod macros;
mod repository;
mod service;

#[cfg(test)]
mod test;

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
            update_project,
            delete_project,
            get_project,
            get_task,
            get_project_tasks,
            set_task_archived,
            set_task_column,
            get_project_file,
            list_project_tree,
            list_project_branches,
            get_project_activity,
            get_project_members,
            add_project_member,
            remove_project_member,
            register,
            login,
            refresh_token,
            update_username,
            get_overlay,
            create_active_overlay,
            wipe_my_overlay,
            get_merge_conflicts,
            ws_overlay_stream,
            ws_project_activity,
            get_user_id_by_username,
            create_org,
            get_org,
            update_org,
            delete_org,
            list_my_orgs,
            list_org_members,
            add_org_member,
            remove_org_member,
            transfer_org_ownership,
            list_org_projects,
        ),
        components(
            schemas(
                TaskRes,
                SetArchivedReq,
                SetColumnReq,
                KanbanColumn,
                TaskType,
                CreateProjectReq,
                ProjectRes,
                LoginPayload,
                RegisterPayload,
                RefreshReq,
                RefreshRes,
                UpdateUsernamePayload,
                UpdateUsernameRes,
                OverlayViewRes,
                UpdateProjectReq,
                ProjectMemberRes,
                AddProjectMemberReq,
                ProjectRole,
                ProjectTreeRes,
                Conflict,
                ActiveEdit,
                UserSearchEntryRes,
                CreateProjectRes,
                CreateOrgReq,
                CreateOrgRes,
                UpdateOrgReq,
                OrgRes,
                MyOrgRes,
                OrgMemberRes,
                OrgRole,
                AddOrgMemberReq,
                TransferOrgOwnershipReq,
                Comment,
            ),
        ),
        modifiers(&UuidSchema),
        security(( "Authorization" = [] )),
        modifiers(&SecuritySchemas),
        tags(
            (name = "project", description = "Project endpoints"),
            (name = "user", description = "User endpoints"),
            (name = "overlay", description = "Overlay endpoints"),
            (name = "task", description = "Task endpoints"),
            (name = "merge", description = "Merge endpoints"),
            (name = "config", description = "Config endpoints"),
            (name = "org", description = "Organization endpoints"),
            (name = "comment", description = "Comment endpoints"),
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
                    .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE"])
                    .allowed_headers(vec!["Content-Type", "Authorization"])
                    .max_age(3600),
            )
            .app_data(app_state.clone())
            .configure(global_routes::init_api_scope)
            .configure(global_routes::init_anon_scope)
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
    // ensure the clone dir exists so the first project create doesnt 404
    tokio::fs::create_dir_all(&repo_loc_path)
        .await
        .expect("Failed to create repo dir");
    let sb_client = init_supabase_db_client();
    let gh_client = env::var("GITHUB_CLIENT_ID").expect("Could not find GITHUB_CLIENT_ID");
    let gh_callback = env::var("GITHUB_CALLBACK_URL").expect("Could not find GITHUB_CALLBACK_URL");
    let gh_secret = env::var("GITHUB_CLIENT_SECRET").expect("Could not find GITHUB_CLIENT_SECRET");

    // construct the JwksCache once; clones share the same Arc<RwLock> state
    let supabase_url = env::var("SUPABASE_URL").expect("Could not find SUPABASE_URL");
    let jwks_url = format!(
        "{}/auth/v1/.well-known/jwks.json",
        supabase_url.trim_end_matches('/'),
    );
    let jwks_cache = supabase_jwt::JwksCache::new(&jwks_url);

    AppState {
        repo_states: DashMap::<Uuid, ProjectLiveState>::new(),
        repo_loc: repo_loc_path,
        sb_client,
        auth_client: init_auth_client(),
        github_client_id: gh_client,
        github_callback_url: gh_callback,
        github_client_secret: gh_secret,
        jwks_cache,
    }
}

fn init_supabase_db_client() -> SupabaseClient {
    supabase_rs::SupabaseClient::new(
        env::var("SUPABASE_URL").expect("Could not find SUPABASE_URL"),
        env::var("SUPABASE_API_KEY").expect("Could not find SUPABASE_ANON_KEY"),
    )
    .expect("Failed initializing Supabase client")
}

fn init_auth_client() -> AuthClient {
    dotenv().ok();

    let url = env::var("SUPABASE_URL").expect("Undefined env: SUPABASE_URL");
    let api_key = env::var("SUPABASE_API_KEY").expect("Undefined env: SUPABASE_API_KEY");
    let anon_key = env::var("SUPABASE_ANON_KEY").expect("Undefined env: SUPABASE_ANON_KEY");

    AuthClient::new(url, api_key, anon_key)
}
