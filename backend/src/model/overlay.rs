use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio::time::Instant;
use utoipa::ToSchema;
use uuid::Uuid;

// Temporary storage for live changes
#[derive(Clone)]
pub struct ProjectLiveState {
    // Key: Filename; Value: Overlay
    pub overlays: DashMap<String, Overlay>,
    // project-wide broadcast: every per-file Change triggers a fresh snapshot
    pub activity_tx: broadcast::Sender<Vec<ActiveEdit>>,
}

#[derive(Clone)]
pub struct Overlay {
    pub original_content: String,
    // Key: User id; Value: their content
    pub user_contents: DashMap<Uuid, UserOverlay>,
    pub tx: broadcast::Sender<WsBroadcast>,
    // In-memory line comments. Lost on restart.
    pub comments: DashMap<Uuid, Comment>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct Comment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub line: u32,
    pub text: String,
    pub created_at: i64,
}

#[derive(Clone)]
pub struct UserOverlay {
    pub content: String,
    pub branch: String,
    pub edited_sections: (u32, u32),
    pub updated_at: Instant,
}

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct OverlayViewRes {
    pub content: String,
    pub original_content: String,
    pub all_user_contents: Vec<UserOverlayRes>,
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct UserOverlayRes {
    pub user_id: Uuid,
    pub content: String,
    pub edited_sections: (u32, u32),
    pub updated_at_secs: u64,
    pub updated_at_nanos: u32,
}

/// Tagged enum carried on the per-file overlay broadcast channel and over
/// the WebSocket wire. Clients dispatch on "kind" to react to typing,
/// resolution suggestions, and comment lifecycle events. Persisting
/// comments through this channel rather than a separate REST + polling
/// loop is the whole point of the refactor.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WsBroadcast {
    /// A user's live typing update for this file.
    Overlay {
        user_id: Uuid,
        content: String,
        line_section: (u32, u32),
    },
    /// A user is proposing this content as a conflict resolution.
    /// Broadcast-only; never mutates any overlay state.
    Suggestion {
        user_id: Uuid,
        content: String,
        line_section: (u32, u32),
    },
    /// A comment was created. id is server-generated; clients sending this
    /// variant fill it with Uuid::nil(), the WS handler replaces it.
    CommentCreated {
        #[serde(default)]
        id: Uuid,
        user_id: Uuid,
        line: u32,
        text: String,
        #[serde(default)]
        created_at: i64,
    },
    /// A comment was removed.
    CommentDeleted { id: Uuid },
    /// Initial snapshot pushed by the server right after a WS subscription
    /// opens, so clients dont need a separate HTTP call to seed comments
    /// and active users. only sent server -> client.
    Snapshot {
        comments: Vec<Comment>,
        all_user_contents: Vec<UserOverlayRes>,
    },
    /// Reply to a Suggestion. The responder accepted or declined; the
    /// suggester reads this off the broadcast to know the outcome.
    /// Broadcast-only on the server side, no state mutation.
    SuggestionResponse {
        suggester_id: Uuid,
        responder_id: Uuid,
        accepted: bool,
    },
    /// A user's session closed (WS disconnect or session stop). The server
    /// removes their entry from user_contents before broadcasting this so
    /// remaining subscribers can drop them from the active-editors view and
    /// stop synthesizing live conflicts against their stale content.
    UserLeft { user_id: Uuid },
}

// A hunk is a contiguous region of changed lines in a diff. Git uses the same term.
// user_id is Some for hunks sourced from a live overlay (so the UI can name
// the editing user), None for hunks read from a branch's committed content.
#[derive(Debug, ToSchema, Serialize, Clone)]
pub struct Hunk {
    pub branch: String,
    #[schema(value_type = Option<String>)]
    pub user_id: Option<Uuid>,
    pub base_start: usize,
    pub base_end: usize,
    pub content: Vec<String>,
}

// TODO: rename to ConflictRes
#[derive(Serialize, ToSchema, Debug)]
pub struct Conflict {
    pub base_start: usize,
    pub base_end: usize,
    pub hunks: Vec<Hunk>, // the conflicting hunks from different branches
}

// One active edit visible in the project-wide activity dashboard.
#[derive(Serialize, ToSchema, Debug, Clone)]
pub struct ActiveEdit {
    pub file: String,
    #[schema(value_type = String)]
    pub user_id: Uuid,
    pub branch: String,
    pub edited_sections: (u32, u32),
}

/// Returns a guard to the overlay stored for "file_name".
/// The guard lives as long as the caller holds it, so no cloning occurs.
pub fn extract_overlay<'a>(
    proj_state: &'a ProjectLiveState,
    file_name: &str,
) -> Option<Ref<'a, String, Overlay>> {
    proj_state.overlays.get(file_name)
}

impl UserOverlay {
    pub fn new(branch: String, content: String) -> Self {
        Self {
            content,
            edited_sections: (0, 0),
            branch,
            updated_at: Instant::now(),
        }
    }
}
