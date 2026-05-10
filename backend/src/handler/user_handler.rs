use crate::error::custom_errors::AuthError;
use crate::model::app_state::AppState;
use crate::model::user::GithubCallbackQuery;
use crate::model::user::GithubTokenResponse;
use crate::model::user::LoginPayload;
use crate::model::user::LoginRes;
use crate::model::user::RegisterPayload;
use crate::repository::user_repository;
use actix_web::HttpResponse;
use actix_web::web;
use log::error;
use log::info;
use serde_json::json;
use supabase_auth::models::SignUpWithPasswordOptions;
use uuid::Uuid;
use validator::Validate;

#[utoipa::path(
    get, 
    path = "/user/{username}", 
    params(("username" = String, Path, example = "tom")),
    tag = "user",
)]
pub async fn get_user_id_by_username(
    username: web::Path<String>,
    state: web::Data<AppState>,
) -> HttpResponse {
    match user_repository::get_user_id_by_username(&state.sb_client, &username.into_inner()).await {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => {
            error!("Failed retrieving user_id by username: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[utoipa::path(post, path = "/register", tag = "user")]
pub async fn register(
    body: web::Json<RegisterPayload>,
    state: web::Data<AppState>,
) -> HttpResponse {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(e);
    }
    

    let options = SignUpWithPasswordOptions {
        data: Some(serde_json::json!({
            "display_name": body.username
        })),
        ..Default::default()
    };
    match state
        .auth_client
        .sign_up_with_email_and_password(&body.email, &body.password, Some(options))
        .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            let auth_error = AuthError::from_supabase_error(&e);
            match auth_error {
                AuthError::EmailAlreadyExists => {
                    HttpResponse::Conflict().json(json!({"error": "Email already registered"}))
                }
                AuthError::UsernameAlreadyTaken => {
                    HttpResponse::Conflict().json(json!({"error": "Username already taken"}))
                }
                AuthError::AlreadyExists => {
                    HttpResponse::Conflict().json(json!({"error": "Already exists"}))
                }
                AuthError::WeakPassword => {
                    HttpResponse::BadRequest().json(json!({"error": "Password too weak"}))
                }
                AuthError::RateLimited => {
                    HttpResponse::TooManyRequests().json(json!({"error": "Too many attempts"}))
                }
                _ => {
                    error!("Registration failed: {e}");
                    HttpResponse::InternalServerError()
                        .json(json!({"error": "Registration failed"}))
                }
            }
        }
    }
}

#[utoipa::path(
    post,
    path = "/login",
    request_body = LoginPayload,
    tag = "user",
)]
pub async fn login(body: web::Json<LoginPayload>, state: web::Data<AppState>) -> HttpResponse {

    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(e);
    }

    let session = match state
        .auth_client
        .login_with_email(&body.email, &body.password)
        .await
    {

        Ok(session) => session,
        Err(e) => {
            let auth_error = AuthError::from_supabase_error(&e);
            match auth_error {
                AuthError::EmailNotConfirmed => {
                    return HttpResponse::Forbidden().json(json!({"error": "Email not confirmed"}));
                }
                AuthError::InvalidCredentials => {
                    return HttpResponse::Unauthorized()
                        .json(json!({"error": "Invalid credentials"}));
                }
                AuthError::RateLimited => {
                    return HttpResponse::TooManyRequests()
                        .json(json!({"error": "Too many attempts"}));
                }
                _ => {
                    error!("Login failed: {e}");
                    return HttpResponse::InternalServerError()
                        .json(json!({"error": "Login failed"}));
                }
            }
        }
    };
    HttpResponse::Ok().json(LoginRes {
        user_id: session.user.id,
        email: body.email.clone(),
        access_token: session.access_token,
    })
}

// GET /auth/github?user_id=<uuid>
// Redirects browser to GitHub's OAuth consent screen
pub async fn github_auth(state: web::Data<AppState>, user_id: web::Path<Uuid>) -> HttpResponse {
    // state.github_client_id comes from env
    // redirect_uri = "{your_api_url}/auth/github/callback"
    // state param = user_id (so you know who to associate the token with on callback)
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=repo&state={}",
        state.github_client_id, state.github_callback_url, user_id
    );
    HttpResponse::Found()
        .insert_header(("Location", url))
        .finish()
}

// GET /auth/github/callback?code=<code>&state=<user_id>
// GitHub redirects here after user consents
pub async fn github_callback(
    state: web::Data<AppState>,
    query: web::Query<GithubCallbackQuery>,
) -> HttpResponse {
    // POST to https://github.com/login/oauth/access_token
    let client = reqwest::Client::new();

    info!("GitHub callback state: '{}'", &query.state);
    let res = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&json!({
            "client_id": state.github_client_id,
            "client_secret": state.github_client_secret,
            "code": query.code
        }))
        .send()
        .await;

    let token: GithubTokenResponse = res.unwrap().json().await.unwrap();
    user_repository::update_access_token(&state.sb_client, &query.state, token.access_token)
        .await
        .unwrap();
    HttpResponse::Ok().finish()
}