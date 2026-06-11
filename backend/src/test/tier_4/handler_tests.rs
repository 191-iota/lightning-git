//! Tier 4, handler contract with mocked permissions.
//!
//! These tests bypass JWT validation via a test-only fake auth middleware
//! that reads "X-Test-User-Id" from the request and injects MiddlewareData
//! directly. The real permission_service still runs and queries Supabase via
//! httpmock, so the actual handler -> permission_service -> mocked-DB path
//! is exercised end to end.

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::middleware::from_fn;
use actix_web::{App, Error, HttpMessage, HttpResponse, http::StatusCode, test, web};
use httpmock::prelude::*;
use uuid::Uuid;

use crate::handler::project_handler::{get_project, list_project_tree};
use crate::model::user::MiddlewareData;
use crate::test::helpers::helpers::test_app_state_with_supabase;

/// Test-only middleware: reads X-Test-User-Id and inserts MiddlewareData so
/// downstream handlers behave as if a real JWT had been validated. No request
/// header => 401, mirroring the real auth_filter's reject behavior.
async fn fake_auth(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let user_id = req
        .headers()
        .get("X-Test-User-Id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok());

    match user_id {
        Some(id) => {
            req.extensions_mut().insert(MiddlewareData { user_id: id });
            Ok(next.call(req).await?.map_into_left_body())
        }
        None => {
            let (req, _) = req.into_parts();
            Ok(ServiceResponse::new(
                req,
                HttpResponse::Unauthorized()
                    .body("test auth: missing X-Test-User-Id")
                    .map_into_right_body(),
            ))
        }
    }
}

/// TF13: a user who is neither an org owner nor a project member is rejected.
/// Supabase mock returns empty rows for both lookups -> permission_service
/// returns Ok(false) -> require_project_permission! returns 401.
#[actix_web::test]
async fn non_member_cannot_read_project() {
    let server = MockServer::start_async().await;
    // is_org_owner_of_project: project lookup returns nothing -> not an owner
    let _project_mock = server
        .mock_async(|when, then| {
            when.method(GET).path("/rest/v1/project");
            then.status(200).body("[]");
        })
        .await;
    // project_members lookup also returns nothing -> not a member
    let _member_mock = server
        .mock_async(|when, then| {
            when.method(GET).path("/rest/v1/project_members");
            then.status(200).body("[]");
        })
        .await;

    let state = web::Data::new(test_app_state_with_supabase(&server.base_url()));
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .wrap(from_fn(fake_auth))
            .route("/projects/{id}", web::get().to(get_project)),
    )
    .await;

    let proj_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{proj_id}"))
        .insert_header(("X-Test-User-Id", user_id.to_string()))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// TF13 variant: the same non-member is also rejected on the tree endpoint.
/// Ensures permission checks aren't accidentally skipped on the read paths
/// that drive the OverlayView sidebar.
#[actix_web::test]
async fn non_member_cannot_list_project_tree() {
    let server = MockServer::start_async().await;
    let _project_mock = server
        .mock_async(|when, then| {
            when.method(GET).path("/rest/v1/project");
            then.status(200).body("[]");
        })
        .await;
    let _member_mock = server
        .mock_async(|when, then| {
            when.method(GET).path("/rest/v1/project_members");
            then.status(200).body("[]");
        })
        .await;

    let state = web::Data::new(test_app_state_with_supabase(&server.base_url()));
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .wrap(from_fn(fake_auth))
            .route("/projects/{id}/tree", web::get().to(list_project_tree)),
    )
    .await;

    let proj_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{proj_id}/tree?branch=main"))
        .insert_header(("X-Test-User-Id", user_id.to_string()))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// TF13 variant: missing user id from the test middleware short-circuits
/// before reaching the handler. Sanity-checks the fake auth itself.
#[actix_web::test]
async fn missing_test_user_id_short_circuits_to_401() {
    let state = web::Data::new(test_app_state_with_supabase("http://localhost"));
    let app = test::init_service(
        App::new()
            .app_data(state.clone())
            .wrap(from_fn(fake_auth))
            .route("/projects/{id}", web::get().to(get_project)),
    )
    .await;

    let proj_id = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/projects/{proj_id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
