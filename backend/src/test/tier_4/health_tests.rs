//! Tier 4, /health endpoint contract.
//!
//! /health is an anonymous route that probes Supabase. Tests use httpmock to
//! stand in for the Supabase REST API and validate that the handler maps
//! Supabase reachability to the right status code + JSON shape.

use actix_web::{App, http::StatusCode, test, web};
use httpmock::prelude::*;
use serde_json::Value;

use crate::routes::global_routes;
use crate::test::helpers::helpers::test_app_state_with_supabase;

/// /health returns 200 with "{"status":"ok","supabase":"up"}" when Supabase
/// responds successfully to the probe SELECT.
#[actix_web::test]
async fn health_check_returns_ok_when_supabase_up() {
    let server = MockServer::start_async().await;
    let _mock = server
        .mock_async(|when, then| {
            when.method(GET).path("/rest/v1/organization");
            then.status(200)
                .header("content-type", "application/json")
                .body("[]");
        })
        .await;

    let state = web::Data::new(test_app_state_with_supabase(&server.base_url()));
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .configure(global_routes::init_anon_scope),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["supabase"], "up");
}

/// /health returns 503 with "status: degraded" when Supabase is unreachable
/// (we point the client at a port nothing is listening on).
#[actix_web::test]
async fn health_check_returns_degraded_when_supabase_down() {
    // 127.0.0.1:1 is the discard port; the connect attempt fails immediately
    // on most systems, which is exactly the Supabase-down case we want to test.
    let state = web::Data::new(test_app_state_with_supabase("http://127.0.0.1:1"));
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .configure(global_routes::init_anon_scope),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "degraded");
    assert_eq!(body["supabase"], "down");
}
