use crate::error::custom_errors::AuthError;
use crate::model::app_state::AppState;
use crate::model::user::GithubCallbackQuery;
use crate::model::user::GithubTokenResponse;
use crate::model::user::LoginPayload;
use crate::model::user::LoginRes;
use crate::model::user::MiddlewareData;
use crate::model::user::RefreshReq;
use crate::model::user::RegisterPayload;
use crate::model::user::UpdateUsernamePayload;
use crate::model::user::UpdateUsernameRes;
use crate::repository::user_repository;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::web;
use log::error;
use log::info;
use log::warn;
use serde_json::json;
use supabase_auth::models::SignUpWithPasswordOptions;
use supabase_auth::models::UpdatedUser;
use uuid::Uuid;
use validator::Validate;

/// Resolve a username to a user id and display name.
/// Used by the member invite flows to look users up by handle.
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

/// Change the authenticated user's username (display name).
///
/// The handle lives in two places: the `profiles.display_name` column the app
/// reads everywhere (member lists, invite-by-username) and the Supabase auth
/// user metadata seeded at signup. We treat the profiles row as the source of
/// truth and write it first, then best-effort sync the auth metadata so a later
/// token refresh carries the same name. The auth update uses the caller's own
/// bearer token (PUT /auth/v1/user updates the current user).
#[utoipa::path(
    patch,
    path = "/api/user/me/username",
    request_body = UpdateUsernamePayload,
    tag = "user",
)]
pub async fn update_username(
    req: HttpRequest,
    body: web::Json<UpdateUsernamePayload>,
    state: web::Data<AppState>,
    ext_data: web::ReqData<MiddlewareData>,
) -> HttpResponse {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(e);
    }

    let user_id = ext_data.user_id;
    let new_name = body.username.trim().to_string();

    // Reject a handle already worn by someone else. The invite flow resolves a
    // user from the first display_name match, so duplicates make invites
    // ambiguous. There is no DB unique constraint, so this is a best-effort
    // check rather than a hard guarantee against races.
    match user_repository::get_user_id_by_username(&state.sb_client, &new_name).await {
        Ok(matches) => {
            if matches.iter().any(|m| m.id != user_id) {
                return HttpResponse::Conflict().json(json!({"error": "Username already taken"}));
            }
        }
        Err(e) => {
            error!("Failed checking username availability: {e}");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Failed updating username"}));
        }
    }

    // Authoritative write: the column the rest of the app reads from.
    if let Err(e) = user_repository::update_display_name(&state.sb_client, &user_id, &new_name).await
    {
        error!("Failed updating display_name: {e}");
        return HttpResponse::InternalServerError()
            .json(json!({"error": "Failed updating username"}));
    }

    // Best-effort: keep the auth user metadata in sync so a future token still
    // matches. The app reads from profiles, so a failure here is logged but
    // does not fail the request.
    if let Some(token) = bearer_token(&req) {
        let payload = UpdatedUser {
            email: None,
            password: None,
            data: Some(json!({ "display_name": new_name })),
        };
        if let Err(e) = state.auth_client.update_user(payload, &token).await {
            warn!("display_name updated in profiles but auth metadata sync failed: {e}");
        }
    } else {
        warn!("display_name updated in profiles but no bearer token to sync auth metadata");
    }

    HttpResponse::Ok().json(UpdateUsernameRes {
        display_name: new_name,
    })
}

/// Pull the raw access token out of the Authorization header, dropping the
/// "Bearer " prefix the Supabase auth client adds back itself.
fn bearer_token(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|t| t.to_string())
}

/// Sign a new user up via Supabase Auth, storing their chosen username
/// in the user metadata for later display in the app.
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

/// Exchange email and password for an access plus refresh token via Supabase.
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
        refresh_token: session.refresh_token,
    })
}

/// Refresh an expired access token using a stored refresh token.
#[utoipa::path(
    post,
    path = "/refresh",
    request_body = RefreshReq,
    tag = "user",
)]
pub async fn refresh_token(
    body: web::Json<RefreshReq>,
    state: web::Data<AppState>,
) -> HttpResponse {
    match state.auth_client.refresh_session(&body.refresh_token).await {
        Ok(session) => HttpResponse::Ok().json(crate::model::user::RefreshRes {
            access_token: session.access_token,
            refresh_token: session.refresh_token,
        }),
        Err(e) => {
            error!("Refresh failed: {e}");
            HttpResponse::Unauthorized().json(json!({"error": "Refresh failed"}))
        }
    }
}

/// Redirect to GitHub's OAuth consent screen with the user id as state.
/// First leg of the flow that lets the backend act on private repos for this user.
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

/// GitHub OAuth callback. Exchanges the code for an access token and stores
/// it against the user id passed through the state parameter.
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