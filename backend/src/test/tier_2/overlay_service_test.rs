//! Tier 2: Overlay service logic.
//!
//! Tests extract_overlay_file_contents and build_overlay_response against a
//! real AppState with real DashMap instances. No HTTP, no filesystem.

use actix_web::web;
use uuid::Uuid;

use crate::service::overlay_service::{build_overlay_response, extract_overlay_file_contents};
use crate::test::helpers::helpers::test_app_state;

// extract_overlay_file_contents

#[test]
fn extract_returns_live_content_not_original() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user = Uuid::new_v4();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user,
        "original git content".into(),
        "feature".into(),
    );

    // Simulate websocket write
    {
        let project = state.repo_states.get(&proj).unwrap();
        let overlay = project.overlays.get("main.rs").unwrap();
        let mut uo = overlay.user_contents.get_mut(&user).unwrap();
        uo.content = "live edited content".to_string();
    }

    let data = web::Data::new(state);
    let result = extract_overlay_file_contents("main.rs".into(), proj, data).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].branch, "feature");
    assert_eq!(result[0].content, "live edited content");
    assert_eq!(result[0].user_id, user);
}

#[test]
fn extract_returns_multiple_users_on_different_branches() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user_a,
        "original".into(),
        "branch-a".into(),
    );
    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user_b,
        "original".into(),
        "branch-b".into(),
    );

    let data = web::Data::new(state);
    let result = extract_overlay_file_contents("main.rs".into(), proj, data).unwrap();

    assert_eq!(result.len(), 2);
    assert!(result.iter().any(|s| s.branch == "branch-a"));
    assert!(result.iter().any(|s| s.branch == "branch-b"));
}

#[test]
fn extract_returns_error_for_missing_project() {
    let state = test_app_state();
    let data = web::Data::new(state);
    let result = extract_overlay_file_contents("main.rs".into(), Uuid::new_v4(), data);
    assert!(result.is_err());
}

#[test]
fn extract_returns_error_for_missing_file() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user = Uuid::new_v4();

    state.get_or_create_overlay(proj, "main.rs".into(), user, "x".into(), "b".into());

    let data = web::Data::new(state);
    let result = extract_overlay_file_contents("nonexistent.rs".into(), proj, data);
    assert!(result.is_err());
}

// build_overlay_response

#[tokio::test]
async fn response_contains_requesting_users_content() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user = Uuid::new_v4();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user,
        "original".into(),
        "feature".into(),
    );

    // Mutate user content
    {
        let project = state.repo_states.get(&proj).unwrap();
        let overlay = project.overlays.get("main.rs").unwrap();
        let mut uo = overlay.user_contents.get_mut(&user).unwrap();
        uo.content = "user edit".to_string();
    }

    let project = state.repo_states.get(&proj).unwrap();
    let overlay = project.overlays.get("main.rs").unwrap();
    let res = build_overlay_response(&overlay, user).await;

    assert_eq!(res.content, "user edit");
    assert_eq!(res.original_content, "original");
}

#[tokio::test]
async fn response_falls_back_to_original_for_unknown_user() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let creator = Uuid::new_v4();
    let stranger = Uuid::new_v4();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        creator,
        "original".into(),
        "feature".into(),
    );

    let project = state.repo_states.get(&proj).unwrap();
    let overlay = project.overlays.get("main.rs").unwrap();
    let res = build_overlay_response(&overlay, stranger).await;

    assert_eq!(
        res.content, "original",
        "unknown user gets original_content"
    );
    assert_eq!(res.original_content, "original");
}

#[tokio::test]
async fn response_includes_all_users() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user_a,
        "original".into(),
        "a".into(),
    );
    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user_b,
        "original".into(),
        "b".into(),
    );

    let project = state.repo_states.get(&proj).unwrap();
    let overlay = project.overlays.get("main.rs").unwrap();
    let res = build_overlay_response(&overlay, user_a).await;

    assert_eq!(res.all_user_contents.len(), 2);
    assert!(res.all_user_contents.iter().any(|u| u.user_id == user_a));
    assert!(res.all_user_contents.iter().any(|u| u.user_id == user_b));
}

#[tokio::test]
async fn response_elapsed_time_is_nonzero_after_delay() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user = Uuid::new_v4();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user,
        "original".into(),
        "feature".into(),
    );

    // Small delay so elapsed > 0
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let project = state.repo_states.get(&proj).unwrap();
    let overlay = project.overlays.get("main.rs").unwrap();
    let res = build_overlay_response(&overlay, user).await;

    let user_res = res
        .all_user_contents
        .iter()
        .find(|u| u.user_id == user)
        .unwrap();
    assert!(
        user_res.updated_at_secs > 0 || user_res.updated_at_nanos > 0,
        "elapsed time should be nonzero after delay"
    );
}
