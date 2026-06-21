//! Tier 1: Merge conflict detection.
//!
//! Tests "compute_conflicts(hunks) -> Vec<Conflict>".
//!
//! A conflict is a group of overlapping hunks from different branches with
//! different signatures. The rules:
//!   - Same branch overlapping itself: not a conflict
//!   - Different branches, same range, same content (same signature): not a conflict
//!   - Different branches, overlapping range, different content: conflict
//!   - Overlapping hunks are merged into a single conflict group (transitive)
//!
//! Needs no Appstate and IO. Is not async and runs in microseconds.

#[cfg(test)]
use crate::model::overlay::Hunk;
use crate::service::merge_service::compute_conflicts;

#[test]
fn no_hunks_produces_no_conflicts() {
    let conflicts = compute_conflicts(vec![]);
    assert!(conflicts.is_empty());
}

#[test]
fn single_branch_never_conflicts_with_itself() {
    let hunks = vec![
        Hunk {
            branch: "b1".to_string(),
            user_id: None,
            base_start: 0,
            base_end: 2,
            content: vec!["x".to_string()],
        },
        Hunk {
            branch: "b1".to_string(),
            user_id: None,
            base_start: 1,
            base_end: 3,
            content: vec!["y".to_string()],
        },
    ];
    let conflicts = compute_conflicts(hunks);
    assert!(conflicts.is_empty());
}

#[test]
fn two_branches_overlapping_different_edits_is_conflict() {
    let hunks = vec![
        Hunk {
            branch: "b1".to_string(),
            user_id: None,
            base_start: 0,
            base_end: 2,
            content: vec!["x".to_string()],
        },
        Hunk {
            branch: "b2".to_string(),
            user_id: None,
            base_start: 1,
            base_end: 3,
            content: vec!["y".to_string()],
        },
    ];
    let conflicts = compute_conflicts(hunks);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].hunks.len(), 2);
}

#[test]
fn two_branches_same_edit_same_range_is_not_conflict() {
    let hunks = vec![
        Hunk {
            branch: "b1".to_string(),
            user_id: None,
            base_start: 0,
            base_end: 2,
            content: vec!["same".to_string()],
        },
        Hunk {
            branch: "b2".to_string(),
            user_id: None,
            base_start: 0,
            base_end: 2,
            content: vec!["same".to_string()],
        },
    ];
    let conflicts = compute_conflicts(hunks);
    assert!(conflicts.is_empty());
}

#[test]
fn adjacent_non_overlapping_hunks_are_not_conflict() {
    let hunks = vec![
        Hunk {
            branch: "b1".to_string(),
            user_id: None,
            base_start: 0,
            base_end: 2,
            content: vec!["x".to_string()],
        },
        Hunk {
            branch: "b2".to_string(),
            user_id: None,
            base_start: 3,
            base_end: 5,
            content: vec!["y".to_string()],
        },
    ];
    let conflicts = compute_conflicts(hunks);
    assert!(conflicts.is_empty());
}

#[test]
fn three_branches_overlapping_produces_single_conflict_group() {
    let hunks = vec![
        Hunk {
            branch: "b1".to_string(),
            user_id: None,
            base_start: 0,
            base_end: 3,
            content: vec!["a".to_string()],
        },
        Hunk {
            branch: "b2".to_string(),
            user_id: None,
            base_start: 2,
            base_end: 5,
            content: vec!["b".to_string()],
        },
        Hunk {
            branch: "b3".to_string(),
            user_id: None,
            base_start: 4,
            base_end: 7,
            content: vec!["c".to_string()],
        },
    ];
    let conflicts = compute_conflicts(hunks);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].hunks.len(), 3);
}

#[test]
fn two_separate_conflict_groups() {
    let hunks = vec![
        Hunk {
            branch: "b1".to_string(),
            user_id: None,
            base_start: 0,
            base_end: 2,
            content: vec!["x".to_string()],
        },
        Hunk {
            branch: "b2".to_string(),
            user_id: None,
            base_start: 0,
            base_end: 2,
            content: vec!["y".to_string()],
        },
        Hunk {
            branch: "b1".to_string(),
            user_id: None,
            base_start: 10,
            base_end: 12,
            content: vec!["a".to_string()],
        },
        Hunk {
            branch: "b2".to_string(),
            user_id: None,
            base_start: 10,
            base_end: 12,
            content: vec!["b".to_string()],
        },
    ];
    let conflicts = compute_conflicts(hunks);
    assert_eq!(conflicts.len(), 2);
}

#[test]
fn boundary_overlap_at_exact_edge() {
    // b2 starts exactly where b1 ends. base_start <= last.1 means this overlaps.
    let hunks = vec![
        Hunk {
            branch: "b1".to_string(),
            user_id: None,
            base_start: 0,
            base_end: 5,
            content: vec!["x".to_string()],
        },
        Hunk {
            branch: "b2".to_string(),
            user_id: None,
            base_start: 5,
            base_end: 8,
            content: vec!["y".to_string()],
        },
    ];
    let conflicts = compute_conflicts(hunks);
    assert_eq!(conflicts.len(), 1);
}
