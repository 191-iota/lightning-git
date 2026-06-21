//! Tier 1: WebSocket conflict wire contract.
//!
//! Pins the cross-repo serialization contract that the TS clients depend on.
//! WsBroadcast is an internally-tagged enum (#[serde(tag = "kind",
//! rename_all = "snake_case")]) and the Conflict/Hunk structs carry
//! snake_case fields. If anyone renames a field or changes the tag, the JS
//! clients silently break; this test fails loudly instead.
//!
//! We assert the presence of the tag value and the field names, plus a
//! Conflict round-trip (it derives Deserialize). We do not assert the full
//! JSON byte-for-byte, so adding a new field stays backwards compatible.

use serde_json::Value;

use crate::model::overlay::{Conflict, Hunk, WsBroadcast};

fn sample_hunk() -> Hunk {
    Hunk {
        branch: "feature".to_string(),
        user_id: None,
        base_start: 2,
        base_end: 4,
        content: vec!["changed line".to_string()],
    }
}

fn sample_conflict() -> Conflict {
    Conflict {
        base_start: 2,
        base_end: 4,
        hunks: vec![sample_hunk()],
    }
}

#[test]
fn conflicts_broadcast_carries_kind_tag_and_field_names() {
    let msg = WsBroadcast::Conflicts {
        file: "doc.txt".to_string(),
        conflicts: vec![sample_conflict()],
    };

    let v: Value = serde_json::to_value(&msg).unwrap();

    // internal tag: the variant discriminator the clients dispatch on.
    assert_eq!(
        v.get("kind").and_then(Value::as_str),
        Some("conflicts"),
        "WsBroadcast::Conflicts must serialize with kind=\"conflicts\""
    );

    // top-level Conflicts fields
    assert_eq!(v.get("file").and_then(Value::as_str), Some("doc.txt"));
    let conflicts = v
        .get("conflicts")
        .and_then(Value::as_array)
        .expect("conflicts must be an array");
    assert_eq!(conflicts.len(), 1);

    // Conflict fields (snake_case)
    let conflict = &conflicts[0];
    assert!(conflict.get("base_start").is_some(), "conflict.base_start");
    assert!(conflict.get("base_end").is_some(), "conflict.base_end");
    let hunks = conflict
        .get("hunks")
        .and_then(Value::as_array)
        .expect("conflict.hunks must be an array");
    assert_eq!(hunks.len(), 1);

    // Hunk fields (snake_case): branch, user_id, base_start, base_end, content
    let hunk = &hunks[0];
    assert!(hunk.get("branch").is_some(), "hunk.branch");
    assert!(
        hunk.as_object().unwrap().contains_key("user_id"),
        "hunk.user_id key must be present even when null"
    );
    assert!(hunk.get("base_start").is_some(), "hunk.base_start");
    assert!(hunk.get("base_end").is_some(), "hunk.base_end");
    assert!(hunk.get("content").is_some(), "hunk.content");
}

#[test]
fn conflict_round_trips_through_serde_json() {
    let original = sample_conflict();

    let json = serde_json::to_string(&original).unwrap();
    let back: Conflict = serde_json::from_str(&json).unwrap();

    // Conflict has no PartialEq derive, so compare the load-bearing fields.
    assert_eq!(back.base_start, original.base_start);
    assert_eq!(back.base_end, original.base_end);
    assert_eq!(back.hunks.len(), original.hunks.len());

    let (a, b) = (&back.hunks[0], &original.hunks[0]);
    assert_eq!(a.branch, b.branch);
    assert_eq!(a.user_id, b.user_id);
    assert_eq!(a.base_start, b.base_start);
    assert_eq!(a.base_end, b.base_end);
    assert_eq!(a.content, b.content);
}
