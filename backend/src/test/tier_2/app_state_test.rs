//! Tier 2: AppState transitions.
//!
//! Tests state management through AppState with real DashMap instances but
//! no HTTP server, no Supabase, no filesystem. Core question: write through
//! path A, read through path B, do I see the write?

use uuid::Uuid;

use crate::test::helpers::helpers::test_app_state;

// get_or_create_overlay: vacant path

#[test]
fn create_overlay_initializes_project_and_file() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user = Uuid::new_v4();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user,
        "fn main() {}".into(),
        "feature-a".into(),
    );

    assert!(state.repo_states.contains_key(&proj));
    let project = state.repo_states.get(&proj).unwrap();
    assert!(project.overlays.contains_key("main.rs"));
}

#[test]
fn create_overlay_sets_user_content_to_initial() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user = Uuid::new_v4();
    let initial = "fn main() {}".to_string();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user,
        initial.clone(),
        "feature-a".into(),
    );

    let project = state.repo_states.get(&proj).unwrap();
    let overlay = project.overlays.get("main.rs").unwrap();
    let uo = overlay.user_contents.get(&user).unwrap();
    assert_eq!(uo.content, initial);
    assert_eq!(uo.branch, "feature-a");
}

#[test]
fn create_overlay_sets_original_content() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user = Uuid::new_v4();
    let initial = "original".to_string();

    state.get_or_create_overlay(proj, "lib.rs".into(), user, initial.clone(), "main".into());

    let project = state.repo_states.get(&proj).unwrap();
    let overlay = project.overlays.get("lib.rs").unwrap();
    assert_eq!(overlay.original_content, initial);
}

// get_or_create_overlay: occupied path (reconnect)
// In the current implementation, reconnect intentionally resets the user's
// in-flight content back to the freshly read base. This test documents that.

#[test]
fn reconnect_resets_user_content_to_fresh_base() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user = Uuid::new_v4();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user,
        "original".into(),
        "feature-a".into(),
    );

    // Simulate a websocket edit by mutating content directly
    {
        let project = state.repo_states.get(&proj).unwrap();
        let overlay = project.overlays.get("main.rs").unwrap();
        let mut uo = overlay.user_contents.get_mut(&user).unwrap();
        uo.content = "user edited this".to_string();
    }

    // Reconnect, same user and file, new base content arrives from git
    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user,
        "fresh from git".into(),
        "feature-b".into(),
    );

    let project = state.repo_states.get(&proj).unwrap();
    let overlay = project.overlays.get("main.rs").unwrap();
    let uo = overlay.user_contents.get(&user).unwrap();
    assert_eq!(uo.content, "fresh from git");
    assert_eq!(uo.branch, "feature-b", "branch should update on reconnect");
    assert_eq!(uo.edited_sections, (0, 0));
}

// Multi-user isolation

#[test]
fn two_users_same_file_independent_content() {
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

    // Mutate user_a's content
    {
        let project = state.repo_states.get(&proj).unwrap();
        let overlay = project.overlays.get("main.rs").unwrap();
        let mut uo = overlay.user_contents.get_mut(&user_a).unwrap();
        uo.content = "edited by a".to_string();
    }

    let project = state.repo_states.get(&proj).unwrap();
    let overlay = project.overlays.get("main.rs").unwrap();
    let uo_a = overlay.user_contents.get(&user_a).unwrap();
    let uo_b = overlay.user_contents.get(&user_b).unwrap();
    assert_eq!(uo_a.content, "edited by a");
    assert_eq!(uo_b.content, "original");
}

// Multiple files in same project

#[test]
fn two_files_same_project_independent_overlays() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user = Uuid::new_v4();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user,
        "main content".into(),
        "b".into(),
    );
    state.get_or_create_overlay(
        proj,
        "lib.rs".into(),
        user,
        "lib content".into(),
        "b".into(),
    );

    let project = state.repo_states.get(&proj).unwrap();
    assert_eq!(project.overlays.len(), 2);

    let main_overlay = project.overlays.get("main.rs").unwrap();
    let lib_overlay = project.overlays.get("lib.rs").unwrap();
    assert_eq!(main_overlay.original_content, "main content");
    assert_eq!(lib_overlay.original_content, "lib content");
}

// get_project_state / get_file_overlay error paths

#[test]
fn get_project_state_returns_error_for_missing_project() {
    let state = test_app_state();
    let result = state.get_project_state(&Uuid::new_v4());
    assert!(result.is_err());
}

#[test]
fn get_file_overlay_returns_error_for_missing_file() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user = Uuid::new_v4();

    state.get_or_create_overlay(proj, "main.rs".into(), user, "x".into(), "b".into());

    let result = state.get_file_overlay(proj, "nonexistent.rs".into());
    assert!(result.is_err());
}

// Clone bug regression: write through AppState, read through get_file_overlay

#[test]
fn write_through_state_visible_through_get_file_overlay() {
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

    // Simulate websocket write directly through AppState (not a clone)
    {
        let project = state.repo_states.get(&proj).unwrap();
        let overlay = project.overlays.get("main.rs").unwrap();
        let mut uo = overlay.user_contents.get_mut(&user).unwrap();
        uo.content = "live edit".to_string();
    }

    // Read through get_file_overlay (which clones)
    let cloned_overlay = state.get_file_overlay(proj, "main.rs".into()).unwrap();
    let uo = cloned_overlay.user_contents.get(&user).unwrap();
    assert_eq!(
        uo.content, "live edit",
        "cloned overlay must reflect writes to AppState"
    );
}

// Broadcast channel

#[test]
fn broadcast_channel_shared_across_users() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    let tx_a =
        state.get_or_create_overlay(proj, "main.rs".into(), user_a, "content".into(), "b".into());
    let tx_b =
        state.get_or_create_overlay(proj, "main.rs".into(), user_b, "content".into(), "b".into());

    // Both senders are clones of the same broadcast channel
    let mut rx = tx_a.subscribe();
    let msg = crate::model::overlay::WsBroadcast::Overlay {
        user_id: user_a,
        content: "test".into(),
        line_section: (0, 1),
    };
    let _ = tx_b.send(msg);
    let received = rx.try_recv();
    assert!(
        received.is_ok(),
        "receiver on tx_a must get message sent via tx_b"
    );
}

// Notbremse reset semantics

#[test]
fn reset_user_overlays_restores_committed_content() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user = Uuid::new_v4();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user,
        "committed".into(),
        "feature".into(),
    );

    // Simulate live typing
    {
        let project = state.repo_states.get(&proj).unwrap();
        let overlay = project.overlays.get("main.rs").unwrap();
        let mut uo = overlay.user_contents.get_mut(&user).unwrap();
        uo.content = "leaked secret".to_string();
        uo.edited_sections = (5, 7);
    }

    let reset_count = state.reset_user_overlays(&proj, &user);
    assert_eq!(reset_count, 1);

    let project = state.repo_states.get(&proj).unwrap();
    let overlay = project.overlays.get("main.rs").unwrap();
    let uo = overlay.user_contents.get(&user).unwrap();
    assert_eq!(uo.content, "committed");
    assert_eq!(uo.edited_sections, (0, 0));
}

#[test]
fn reset_user_overlays_only_touches_the_caller() {
    let state = test_app_state();
    let proj = Uuid::new_v4();
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user_a,
        "committed".into(),
        "a".into(),
    );
    state.get_or_create_overlay(
        proj,
        "main.rs".into(),
        user_b,
        "committed".into(),
        "b".into(),
    );

    // Both users have live edits
    {
        let project = state.repo_states.get(&proj).unwrap();
        let overlay = project.overlays.get("main.rs").unwrap();
        overlay.user_contents.get_mut(&user_a).unwrap().content = "a edits".to_string();
        overlay.user_contents.get_mut(&user_b).unwrap().content = "b edits".to_string();
    }

    state.reset_user_overlays(&proj, &user_a);

    let project = state.repo_states.get(&proj).unwrap();
    let overlay = project.overlays.get("main.rs").unwrap();
    let uo_a = overlay.user_contents.get(&user_a).unwrap();
    let uo_b = overlay.user_contents.get(&user_b).unwrap();
    assert_eq!(uo_a.content, "committed");
    assert_eq!(uo_b.content, "b edits", "other users must be untouched");
}
