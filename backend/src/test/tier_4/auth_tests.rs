//! Tier 4, auth_filter middleware contract.
//!
//! Verifies that protected routes reject requests without a valid token.
//! Does NOT mock JWT/JWKS: tests rely on the auth_filter short-circuiting
//! when the Authorization header / token query param is missing or malformed,
//! which happens before any JWKS fetch.

use actix_web::{App, http::StatusCode, test, web};
use uuid::Uuid;

use crate::routes::global_routes;
use crate::test::helpers::helpers::test_app_state;

/// TF15: GET on a protected /api route without an Authorization header
/// must be rejected by the auth filter with 401.
#[actix_web::test]
async fn protected_route_without_auth_returns_401() {
    let state = web::Data::new(test_app_state());
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .configure(global_routes::init_api_scope),
    )
    .await;

    let proj_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/projects/{proj_id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// TF15: GET on a protected /api route with a malformed Authorization header
/// must be rejected with 401 by the auth filter.
#[actix_web::test]
async fn protected_route_with_malformed_auth_returns_401() {
    let state = web::Data::new(test_app_state());
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .configure(global_routes::init_api_scope),
    )
    .await;

    let proj_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/projects/{proj_id}"))
        .insert_header(("Authorization", "Bearer not-a-real-token"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// TF15: GET on the per-file overlay WS route without a token query param
/// must be rejected before the websocket handshake completes.
#[actix_web::test]
async fn ws_overlay_route_without_token_returns_401() {
    let state = web::Data::new(test_app_state());
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .configure(global_routes::init_api_scope),
    )
    .await;

    let proj_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/overlay/ws/{proj_id}/{user_id}/src/main.rs"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// TF15: GET on the project activity WS without a token query param must
/// be rejected by the auth filter, same as any other /api route.
#[actix_web::test]
async fn ws_project_activity_route_without_token_returns_401() {
    let state = web::Data::new(test_app_state());
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .configure(global_routes::init_api_scope),
    )
    .await;

    let proj_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/projects/{proj_id}/activity/ws"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
