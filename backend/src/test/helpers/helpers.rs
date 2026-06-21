//! Shared test setup. Builds an AppState with real DashMap internals and
//! dummy external clients so Tier 2 tests can exercise the in-memory state
//! machine without hitting Supabase, GitHub or the filesystem.

use crate::model::app_state::AppState;
use dashmap::DashMap;
use std::path::PathBuf;
use supabase_auth::models::AuthClient;
use supabase_jwt::JwksCache;
use supabase_rs::SupabaseClient;

pub fn test_app_state() -> AppState {
    test_app_state_with_supabase("http://localhost")
}

/// Variant for Tier 4 tests that point SupabaseClient at an httpmock server so
/// permission_service calls return canned rows instead of real HTTP failures.
pub fn test_app_state_with_supabase(supabase_url: &str) -> AppState {
    AppState {
        repo_states: DashMap::new(),
        sb_client: SupabaseClient::new(supabase_url, "fake").unwrap(),
        repo_loc: PathBuf::from("/tmp/test-repos"),
        auth_client: AuthClient::new(
            "http://localhost".to_string(),
            "fake".to_string(),
            "fake".to_string(),
        ),
        github_client_id: "fake".to_string(),
        github_callback_url: "fake".to_string(),
        github_client_secret: "fake".to_string(),
        jwks_cache: JwksCache::new("http://localhost/jwks.json"),
    }
}
