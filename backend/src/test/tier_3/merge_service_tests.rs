//! Tier 3: merge_service::calculate_live_diff against a real git repo.
//!
//! This is the product's heart: predicting merge conflicts across branches.
//! It is exercised end-to-end here against real temporary repositories built
//! with the same subprocess pattern as git_service_tests.rs (no git mocking).
//!
//! The invariants pinned here are the contractually stable ones:
//!   - two branches that change the same base line to DIFFERENT content
//!     produce a non-empty conflict set whose range covers that line
//!   - two branches making the IDENTICAL edit produce NO conflict
//!   - a file that does not exist on main (a draft) yields Ok(empty)
//!
//! We deliberately do not assert exact hunk counts, exact byte ranges, or
//! error variants: those are implementation detail and expected to move.

use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::process::Command;
use uuid::Uuid;

use actix_web::web;

use crate::service::merge_service::calculate_live_diff;
use crate::test::helpers::helpers::test_app_state;

// ── Helpers (mirror git_service_tests.rs) ──────────────────────────────

/// Runs a git command in the given directory. Panics on failure.
async fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .await
        .expect("failed to spawn git");
    assert!(
        out.status.success(),
        "git {} failed in {}: {}",
        args.join(" "),
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Writes a file relative to "dir" and returns the full path.
async fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let p = dir.join(name);
    if let Some(parent) = p.parent() {
        tokio::fs::create_dir_all(parent).await.unwrap();
    }
    tokio::fs::write(&p, content).await.unwrap();
    p
}

/// Bare remote + working clone with an initial commit on main, same as the
/// git_service_tests setup. The base content of `file` on main is seeded so
/// branches can diverge from a known base.
struct TestRepo {
    _root: TempDir,
    remote: PathBuf,
    working: PathBuf,
}

impl TestRepo {
    /// Initialise a repo whose `main` already contains `file` with `base`.
    async fn with_base(file: &str, base: &str) -> Self {
        let root = TempDir::new().unwrap();

        let remote = root.path().join("remote.git");
        tokio::fs::create_dir_all(&remote).await.unwrap();
        git(&remote, &["init", "--bare", "--initial-branch=main"]).await;

        let working = root.path().join("working");
        git(root.path(), &["clone", remote.to_str().unwrap(), "working"]).await;

        git(&working, &["config", "user.email", "test@test.com"]).await;
        git(&working, &["config", "user.name", "Test"]).await;

        write_file(&working, file, base).await;
        git(&working, &["add", "."]).await;
        git(&working, &["commit", "-m", "init"]).await;
        git(&working, &["branch", "-M", "main"]).await;
        git(&working, &["push", "-u", "origin", "main"]).await;

        Self {
            _root: root,
            remote,
            working,
        }
    }

    /// Branch from main, overwrite `file` with `content`, commit and push.
    async fn push_branch(&self, branch: &str, file: &str, content: &str) {
        git(&self.working, &["checkout", "-b", branch]).await;
        write_file(&self.working, file, content).await;
        git(&self.working, &["add", "."]).await;
        git(
            &self.working,
            &["commit", "-m", &format!("commit on {branch}")],
        )
        .await;
        git(&self.working, &["push", "-u", "origin", branch]).await;
        git(&self.working, &["checkout", "main"]).await;
    }

    fn remote_url(&self) -> String {
        self.remote.to_str().unwrap().to_string()
    }
}

/// Clone the remote into a fresh SUT dir, the way lightning-git stores a repo.
/// Returns (dst_root kept alive, dst path) so the caller drops the tempdir.
async fn clone_sut(remote_url: &str) -> (TempDir, PathBuf) {
    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    crate::service::git_service::clone_repo(remote_url, &dst)
        .await
        .unwrap();
    (dst_root, dst)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn divergent_branch_edits_to_same_line_conflict() {
    // base has three lines; both branches rewrite the SECOND line differently.
    let base = "line one\nshared base line\nline three\n";
    let repo = TestRepo::with_base("doc.txt", base).await;
    repo.push_branch("alpha", "doc.txt", "line one\nalpha edit\nline three\n")
        .await;
    repo.push_branch("beta", "doc.txt", "line one\nbeta edit\nline three\n")
        .await;

    let (_dst_root, dst) = clone_sut(&repo.remote_url()).await;

    let state = test_app_state();
    let proj = Uuid::new_v4();
    // Project/file overlay must exist so the merge service can collect (the
    // empty set of) live overlays without erroring. No user => conflict is
    // sourced purely from the two committed branches.
    state.ensure_file_overlay(proj, "doc.txt");
    let data = web::Data::new(state);

    let conflicts = calculate_live_diff("doc.txt".into(), proj, data, &dst)
        .await
        .unwrap();

    assert!(
        !conflicts.is_empty(),
        "divergent edits to the same base line must produce a conflict"
    );
    // The changed line is index 1 (0-based) in the base. Its range must be
    // covered by at least one reported conflict cluster. We assert coverage
    // of the line rather than an exact (start,end) so the test survives diff
    // boundary tweaks.
    let changed_line = 1usize;
    assert!(
        conflicts
            .iter()
            .any(|c| c.base_start <= changed_line && changed_line <= c.base_end),
        "no conflict range covers the diverged line {changed_line}: {conflicts:?}"
    );
}

#[tokio::test]
async fn identical_branch_edits_no_conflict() {
    // both branches make the EXACT same edit to the same line => not a conflict.
    let base = "line one\nshared base line\nline three\n";
    let same = "line one\nidentical edit\nline three\n";
    let repo = TestRepo::with_base("doc.txt", base).await;
    repo.push_branch("alpha", "doc.txt", same).await;
    repo.push_branch("beta", "doc.txt", same).await;

    let (_dst_root, dst) = clone_sut(&repo.remote_url()).await;

    let state = test_app_state();
    let proj = Uuid::new_v4();
    state.ensure_file_overlay(proj, "doc.txt");
    let data = web::Data::new(state);

    let conflicts = calculate_live_diff("doc.txt".into(), proj, data, &dst)
        .await
        .unwrap();

    assert!(
        conflicts.is_empty(),
        "identical edits on two branches must not conflict: {conflicts:?}"
    );
}

#[tokio::test]
async fn draft_file_absent_from_main_yields_empty() {
    // The repo's main has no `draft.txt`. A draft has no merge target, so the
    // service must return Ok(empty) rather than erroring.
    let repo = TestRepo::with_base("doc.txt", "only this file\n").await;
    let (_dst_root, dst) = clone_sut(&repo.remote_url()).await;

    let state = test_app_state();
    let proj = Uuid::new_v4();
    let data = web::Data::new(state);

    let conflicts = calculate_live_diff("draft.txt".into(), proj, data, &dst)
        .await
        .unwrap();

    assert!(
        conflicts.is_empty(),
        "a draft file absent from main must yield an empty conflict set"
    );
}
