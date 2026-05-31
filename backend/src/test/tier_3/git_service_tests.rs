//! Tier 3: Filesystem / Subprocess (Integration)
//!
//! Tests git operations against real git subprocesses and temporary repositories.
//! No mocking of git. The value is confirming that argument ordering, output parsing,
//! and error handling work against a real git binary.
//!
//! Requires git installed on the CI runner.
//!
//! Setup pattern:
//!   1. Create bare "remote" repo in a TempDir
//!   2. Clone it into a "working" dir, add files, commit, push branches
//!   3. Clone again into the "system under test" dir (simulates what lightning-git stores)
//!   4. Run git_service functions against the SUT dir
//!   5. TempDir drops automatically on test exit

use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::process::Command;

use crate::service::git_service;

// ── Helpers ────────────────────────────────────────────────────────────

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

/// Creates a bare remote repo + a working clone with an initial commit on main.
/// Returns (remote_dir, working_dir), both inside the TempDir so they drop together.
struct TestRepo {
    _root: TempDir,
    /// Bare remote, this is what clone_repo would point at
    remote: PathBuf,
    /// Working copy used to push commits/branches
    working: PathBuf,
}

impl TestRepo {
    async fn new() -> Self {
        let root = TempDir::new().unwrap();

        let remote = root.path().join("remote.git");
        tokio::fs::create_dir_all(&remote).await.unwrap();
        // force main as default branch independent of host init.defaultBranch
        git(&remote, &["init", "--bare", "--initial-branch=main"]).await;

        let working = root.path().join("working");
        git(root.path(), &["clone", remote.to_str().unwrap(), "working"]).await;

        // git needs user identity for commits
        git(&working, &["config", "user.email", "test@test.com"]).await;
        git(&working, &["config", "user.name", "Test"]).await;

        // initial commit on main, defensively rename the working branch in
        // case the local clone defaulted to master from a stale config
        write_file(&working, "README.md", "# init\n").await;
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

    /// Create a branch with a file change and push it
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
        // go back to main so next branch forks from main
        git(&self.working, &["checkout", "main"]).await;
    }

    /// Returns the remote URL as a string (file:// path to bare repo)
    fn remote_url(&self) -> String {
        self.remote.to_str().unwrap().to_string()
    }
}

// ── clone_repo ─────────────────────────────────────────────────────────

#[tokio::test]
async fn clone_creates_functional_repo() {
    let repo = TestRepo::new().await;
    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");

    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    assert!(dst.join(".git").exists());
    let readme = tokio::fs::read_to_string(dst.join("README.md"))
        .await
        .unwrap();
    assert_eq!(readme, "# init\n");
}

#[tokio::test]
async fn clone_invalid_src_returns_error() {
    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");

    let result = git_service::clone_repo("/nonexistent/repo.git", &dst).await;
    assert!(result.is_err());
}

// ── fetch ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn fetch_succeeds_on_valid_repo() {
    let repo = TestRepo::new().await;
    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    // push a new branch after clone
    repo.push_branch("new-branch", "file.txt", "content").await;

    // fetch should pick it up without error
    git_service::fetch(&dst).await.unwrap();

    // verify the new branch ref exists
    let branches = git_service::list_remote_branches(&dst).await.unwrap();
    assert!(branches.contains("new-branch"));
}

// ── list_remote_branches

#[tokio::test]
async fn list_branches_returns_all_remote_branches() {
    let repo = TestRepo::new().await;
    repo.push_branch("feature-a", "a.txt", "a").await;
    repo.push_branch("feature-b", "b.txt", "b").await;

    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    let branches = git_service::list_remote_branches(&dst).await.unwrap();

    assert!(branches.contains("main"));
    assert!(branches.contains("feature-a"));
    assert!(branches.contains("feature-b"));
    assert_eq!(branches.len(), 3);
}

#[tokio::test]
async fn list_branches_excludes_head_pointer() {
    let repo = TestRepo::new().await;
    repo.push_branch("feature-a", "a.txt", "a").await;

    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    let branches = git_service::list_remote_branches(&dst).await.unwrap();

    // HEAD -> origin/main should be filtered out
    for b in &branches {
        assert!(!b.contains("HEAD"), "HEAD ref leaked into branch list: {b}");
        assert!(!b.contains("->"), "arrow ref leaked into branch list: {b}");
    }
}

#[tokio::test]
async fn list_branches_strips_origin_prefix() {
    let repo = TestRepo::new().await;
    repo.push_branch("feature-x", "x.txt", "x").await;

    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    let branches = git_service::list_remote_branches(&dst).await.unwrap();

    for b in &branches {
        assert!(
            !b.starts_with("origin/"),
            "origin/ prefix not stripped: {b}"
        );
    }
}

#[tokio::test]
async fn list_branches_single_branch_repo() {
    let repo = TestRepo::new().await;

    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    let branches = git_service::list_remote_branches(&dst).await.unwrap();

    assert_eq!(branches.len(), 1);
    assert!(branches.contains("main"));
}

// ── read_file ──────────────────────────────────────────────────────────

#[tokio::test]
async fn read_file_returns_branch_content() {
    let repo = TestRepo::new().await;
    repo.push_branch("feature-a", "src/main.rs", "fn main() {}")
        .await;

    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    let content = git_service::read_file(&dst, "feature-a", Path::new("src/main.rs"))
        .await
        .unwrap();

    assert_eq!(content, "fn main() {}");
}

#[tokio::test]
async fn read_file_main_branch() {
    let repo = TestRepo::new().await;

    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    let content = git_service::read_file(&dst, "main", Path::new("README.md"))
        .await
        .unwrap();

    assert_eq!(content, "# init\n");
}

#[tokio::test]
async fn read_file_nonexistent_branch_returns_error() {
    let repo = TestRepo::new().await;

    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    let result = git_service::read_file(&dst, "ghost-branch", Path::new("README.md")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn read_file_nonexistent_file_returns_error() {
    let repo = TestRepo::new().await;

    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    let result = git_service::read_file(&dst, "main", Path::new("does_not_exist.rs")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn read_file_different_content_per_branch() {
    let repo = TestRepo::new().await;
    repo.push_branch("branch-a", "shared.txt", "version A")
        .await;
    // go back to main from branch-a, then create branch-b
    repo.push_branch("branch-b", "shared.txt", "version B")
        .await;

    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    let a = git_service::read_file(&dst, "branch-a", Path::new("shared.txt"))
        .await
        .unwrap();
    let b = git_service::read_file(&dst, "branch-b", Path::new("shared.txt"))
        .await
        .unwrap();

    assert_eq!(a, "version A");
    assert_eq!(b, "version B");
}

// ── delete_repo ────────────────────────────────────────────────────────

#[tokio::test]
async fn delete_repo_removes_directory() {
    let repo = TestRepo::new().await;
    let dst_root = TempDir::new().unwrap();
    let dst = dst_root.path().join("cloned");
    git_service::clone_repo(&repo.remote_url(), &dst)
        .await
        .unwrap();

    assert!(dst.exists());
    git_service::delete_repo(&dst).await.unwrap();
    assert!(!dst.exists());
}

#[tokio::test]
async fn delete_repo_nonexistent_returns_error() {
    let result = git_service::delete_repo(Path::new("/tmp/definitely_does_not_exist_12345")).await;
    assert!(result.is_err());
}
