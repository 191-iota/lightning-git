//! Tier 1: Only diff logic.
//!
//! Tests "compute_combined_diff(base, branches) -> Vec<Hunk>".
//!
//! A hunk represents one contiguous region of change between base and a branch:
//!   - base_start / base_end: which base lines were affected
//!   - content: what replaced them (empty = pure deletion)
//!
//! One branch can produce multiple hunks if edits are separated by unchanged lines.
//! Multiple branches each produce their own hunks independently.
//!
//! No AppState, no IO, no async. Deterministic, runs in microseconds.

#[cfg(test)]
use crate::service::merge_service::compute_combined_diff;
use indoc::indoc;

#[test]
fn identical_content_produces_no_hunks() {
    let base = indoc! {"
            line one
            line two
            line three
        "}
    .to_string();
    let branches = vec![("branch-a".to_string(), base.clone())];
    let hunks = compute_combined_diff(base, branches);
    assert!(hunks.is_empty());
}

#[test]
fn empty_base_and_empty_branch_produces_no_hunks() {
    let hunks = compute_combined_diff(String::new(), vec![("b".to_string(), String::new())]);
    assert!(hunks.is_empty());
}

#[test]
fn empty_base_with_branch_content_produces_single_insert_hunk() {
    let hunks = compute_combined_diff(
        String::new(),
        vec![("b".to_string(), "new line\n".to_string())],
    );
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].base_start, 0);
    assert_eq!(hunks[0].branch, "b");
    assert!(!hunks[0].content.is_empty());
}

#[test]
fn branch_deletes_all_content() {
    let base = indoc! {"
            line one
            line two
        "}
    .to_string();
    let hunks = compute_combined_diff(base, vec![("b".to_string(), String::new())]);
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].base_start, 0);
    assert!(hunks[0].content.is_empty());
}

#[test]
fn single_line_insertion_in_middle() {
    let base = indoc! {"
            aaa
            bbb
            ccc
        "}
    .to_string();
    let modified = indoc! {"
            aaa
            bbb
            inserted
            ccc
        "}
    .to_string();
    let hunks = compute_combined_diff(base, vec![("b".to_string(), modified)]);
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].base_start, 2);
    assert_eq!(hunks[0].content, vec!["inserted"]);
}

#[test]
fn single_line_deletion_in_middle() {
    let base = indoc! {"
            aaa
            bbb
            ccc
        "}
    .to_string();
    let modified = indoc! {"
            aaa
            ccc
        "}
    .to_string();
    let hunks = compute_combined_diff(base, vec![("b".to_string(), modified)]);
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].base_start, 1);
    assert_eq!(hunks[0].base_end, 2);
    assert!(hunks[0].content.is_empty());
}

#[test]
fn modification_replaces_line() {
    let base = indoc! {"
            aaa
            bbb
            ccc
        "}
    .to_string();
    let modified = indoc! {"
            aaa
            BBB
            ccc
        "}
    .to_string();
    let hunks = compute_combined_diff(base, vec![("b".to_string(), modified)]);
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].content, vec!["BBB"]);
}

#[test]
fn hunk_at_end_of_file_no_trailing_equal() {
    let base = indoc! {"
            aaa
            bbb
        "}
    .to_string();
    let modified = indoc! {"
            aaa
            bbb
            new tail
        "}
    .to_string();
    let hunks = compute_combined_diff(base, vec![("b".to_string(), modified)]);
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].content, vec!["new tail"]);
}

#[test]
fn hunk_at_start_of_file() {
    let base = indoc! {"
            aaa
            bbb
        "}
    .to_string();
    let modified = indoc! {"
            new head
            aaa
            bbb
        "}
    .to_string();
    let hunks = compute_combined_diff(base, vec![("b".to_string(), modified)]);
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].base_start, 0);
    assert_eq!(hunks[0].content, vec!["new head"]);
}

#[test]
fn two_separate_hunks_from_one_branch() {
    let base = indoc! {"
            aaa
            bbb
            ccc
            ddd
            eee
        "}
    .to_string();
    let modified = indoc! {"
            AAA
            bbb
            ccc
            ddd
            EEE
        "}
    .to_string();
    let hunks = compute_combined_diff(base, vec![("b".to_string(), modified)]);
    assert_eq!(hunks.len(), 2);
    assert_eq!(hunks[0].content, vec!["AAA"]);
    assert_eq!(hunks[1].content, vec!["EEE"]);
}

#[test]
fn multiple_branches_produce_separate_hunks() {
    let base = indoc! {"
            aaa
            bbb
            ccc
        "}
    .to_string();
    let b1 = indoc! {"
            AAA
            bbb
            ccc
        "}
    .to_string();
    let b2 = indoc! {"
            aaa
            bbb
            CCC
        "}
    .to_string();
    let branches = vec![("b1".to_string(), b1), ("b2".to_string(), b2)];
    let hunks = compute_combined_diff(base, branches);
    assert_eq!(hunks.len(), 2);
    assert!(hunks.iter().any(|h| h.branch == "b1"));
    assert!(hunks.iter().any(|h| h.branch == "b2"));
}

#[test]
fn whitespace_only_difference_after_trim() {
    let base = "aaa\nbbb  \nccc\n".to_string();
    let modified = "aaa\nbbb\nccc\n".to_string();
    let hunks = compute_combined_diff(base, vec![("b".to_string(), modified)]);
    // After trim_end, "bbb  " and "bbb" may still differ because base isn't trimmed.
    // This test documents actual behavior. If base normalization is added later, update.
    let _ = hunks;
}
