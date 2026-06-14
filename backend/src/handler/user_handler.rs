use crate::error::custom_errors::AuthError;
use crate::model::app_state::AppState;
use crate::model::user::GithubCallbackQuery;
use crate::model::user::GithubTokenResponse;
use crate::model::user::LoginPayload;
use crate::model::user::LoginRes;
use crate::model::user::RefreshReq;
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
            "code": query.code,
            // Must match the redirect_uri used in github_auth or GitHub rejects it.
            "redirect_uri": state.github_callback_url
        }))
        .send()
        .await;

    // Never unwrap on the network call: a failed request must not kill the worker.
    let resp = match res {
        Ok(r) => r,
        Err(e) => {
            error!("GitHub token exchange request failed: {e}");
            return HttpResponse::BadGateway()
                .body("Could not reach GitHub. Please retry the authorization.");
        }
    };

    let status = resp.status();
    // Read the body as text FIRST so we can log GitHub's real error instead of
    // panicking on a missing `access_token` field.
    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed reading GitHub token response body: {e}");
            return HttpResponse::BadGateway()
                .body("Invalid response from GitHub. Please retry the authorization.");
        }
    };

    match serde_json::from_str::<GithubTokenResponse>(&body) {
        // Happy path: we got a token. Store it BEFORE anything can fail so the
        // clone step can later read it back. A DB failure here is logged, not panicked.
        Ok(token) => {
            if let Err(e) =
                user_repository::update_access_token(&state.sb_client, &query.state, token.access_token)
                    .await
            {
                error!("GitHub token obtained but DB update failed for user {}: {e}", &query.state);
                return HttpResponse::InternalServerError()
                    .body("Authorized with GitHub but failed to save the token. Please retry.");
            }
            info!("GitHub token stored for user {}", &query.state);
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(GITHUB_OK_HTML)
        }
        // GitHub did not return a token (e.g. bad_verification_code). Log the real
        // reason, then handle the single-use-code / duplicate-callback race.
        Err(_) => {
            let gh_err = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
                .unwrap_or_else(|| "unknown_error".to_string());
            error!(
                "GitHub token exchange returned no access_token (status {status}, error '{gh_err}'). Raw body: {body}"
            );

            // The OAuth code is single-use: when GitHub redirects the callback more
            // than once, the first request consumes the code and stores the token,
            // and the rest land here with bad_verification_code. If a token is
            // already on file for this user, that race already succeeded.
            if let Ok(uid) = Uuid::parse_str(&query.state) {
                if let Ok(Some(_)) =
                    user_repository::get_access_token(&state.sb_client, &uid).await
                {
                    info!(
                        "Token already stored for user {} (duplicate callback) — treating as success",
                        &query.state
                    );
                    return HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .body(GITHUB_OK_HTML);
                }
            }

            HttpResponse::BadRequest()
                .content_type("text/html; charset=utf-8")
                .body(format!(
                    "<!doctype html><html><body style=\"font-family:system-ui;text-align:center;padding-top:3rem\">\
                     <h2>GitHub authorization failed</h2>\
                     <p>GitHub said: <code>{gh_err}</code>.</p>\
                     <p>The authorization code is single-use and expires after a few minutes.<br>\
                     Please click <b>Authorize on GitHub</b> again and complete it in one go.</p>\
                     </body></html>"
                ))
        }
    }
}

/// Shown in the OAuth popup tab after the token is stored. Auto-closes so the
/// user lands back on the project-create form instead of a blank page.
const GITHUB_OK_HTML: &str = "<!doctype html><html><body style=\"font-family:system-ui;text-align:center;padding-top:3rem\">\
     <h2>GitHub authorized</h2>\
     <p>Lightning Git can now clone your private repositories.<br>You can close this tab and return to the app.</p>\
     <script>setTimeout(function(){window.close()},1500)</script>\
     </body></html>";