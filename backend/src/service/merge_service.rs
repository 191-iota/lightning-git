use actix_web::web;
use futures_util::stream;
use similar::ChangeTag;
use similar::TextDiff;
use std::cmp;
use std::collections::HashSet;
use std::path::Path;
use stream::StreamExt;
use uuid::Uuid;

use crate::error::custom_errors::LGitIoError;
use crate::model::app_state::AppState;
use crate::model::overlay::Conflict;
use crate::model::overlay::Hunk;

use super::git_service;
use super::overlay_service::extract_overlay_file_contents;

/// Predict merge conflicts on one file across all live branches.
/// Diffs each branch (live overlay or committed) against main and reports overlapping hunks.
/// Known limitation: stale-but-not-deleted branches still get scanned each call.
/// Delete merged branches to keep this fast.
pub async fn calculate_live_diff(
    file_name: String,
    project_id: Uuid,
    state: web::Data<AppState>,
    base: &Path,
) -> Result<Vec<Conflict>, LGitIoError> {
    let file_path = Path::new(&file_name);

    // 1. Retrieve base content from the base branch (read_file already fetched origin).
    // If the file is a draft (not yet committed to main), there is no merge target,
    // so there are no conflicts to report.
    let base_content = match git_service::read_file(base, "main", file_path).await {
        Ok(s) => s,
        Err(_) => return Ok(Vec::new()),
    };

    // piggyback the cache refresh: now that we have fresh main, update the overlay's
    // cached original_content and re-flag divergence so the activity view stays honest
    // after pushes. Then push a fresh snapshot to activity subscribers.
    state.refresh_overlay_base(&project_id, &file_name, base_content.clone());
    let activity_tx = state
        .repo_states
        .get(&project_id)
        .map(|p| p.activity_tx.clone());
    if let Some(tx) = activity_tx {
        let _ = tx.send(state.compute_activity(&project_id));
    }

    let mut active_branches = git_service::list_remote_branches(base).await?;

    let file_overlays = extract_overlay_file_contents(file_name.clone(), project_id, state)?;
    // (branch, user_id, content) — user_id is Some only for live-overlay rows.
    let mut sources: Vec<(String, Option<uuid::Uuid>, String)> = Vec::new();
    file_overlays.into_iter().for_each(|f| {
        if active_branches.contains(&f.branch) {
            active_branches.remove(&f.branch);
        }
        sources.push((f.branch, Some(f.user_id), f.content));
    });

    // A branch can legitimately not contain this file (added on the current
    // branch only), so a read miss is skipped, not unwrap-panicked.
    let mut branch_sources: Vec<(String, Option<uuid::Uuid>, String)> = stream::iter(active_branches.iter().cloned())
        .map(|branch: String| async move {
            match git_service::read_file(base, branch.as_str(), file_path).await {
                Ok(content) => Some((branch, None, content)),
                Err(_) => None,
            }
        })
        .buffer_unordered(10)
        .filter_map(|r| async move { r })
        .collect::<Vec<(String, Option<uuid::Uuid>, String)>>()
        .await;

    sources.append(&mut branch_sources);

    let diff = compute_combined_diff(base_content, sources);
    let conflicts = compute_conflicts(diff);

    Ok(conflicts)
}

/// Decompose each source's diff against base into contiguous hunks of changed
/// lines. Each hunk is tagged with the branch and (for live overlays) the
/// editing user id, so the UI can attribute who proposed what.
pub fn compute_combined_diff(
    base_content: String,
    sources: Vec<(String, Option<uuid::Uuid>, String)>,
) -> Vec<Hunk> {
    let mut all_hunks: Vec<Hunk> = Vec::new();

    for (branch, user_id, content) in sources {
        if content == base_content {
            continue;
        }
        // an empty-content source (file deleted in the branch or never
        // populated) shows up as a "removed" hunk and clutters the conflict
        // UI without adding information; the eventual merge will surface the
        // modify/delete case via git itself.
        if content.trim().is_empty() {
            continue;
        }

        let diff = TextDiff::from_lines(&base_content, &content);
        let mut base_line = 0;
        let mut hunk_start = None;
        let mut changed_lines = Vec::new();

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    if let Some(start) = hunk_start.take() {
                        all_hunks.push(Hunk {
                            branch: branch.clone(),
                            user_id,
                            base_start: start,
                            base_end: base_line,
                            content: std::mem::take(&mut changed_lines),
                        });
                    }
                    base_line += 1;
                }
                ChangeTag::Insert => {
                    if hunk_start.is_none() {
                        hunk_start = Some(base_line);
                    }
                    changed_lines.push(change.to_string().trim_end().to_string());
                }
                ChangeTag::Delete => {
                    if hunk_start.is_none() {
                        hunk_start = Some(base_line);
                    }
                    base_line += 1;
                }
            }
        }
        if let Some(start) = hunk_start {
            all_hunks.push(Hunk {
                branch: branch.clone(),
                user_id,
                base_start: start,
                base_end: base_line,
                content: std::mem::take(&mut changed_lines),
            });
        }
    }
    all_hunks
}

/// Group hunks whose base line ranges overlap into Conflict clusters.
/// Transitive: if A overlaps B and B overlaps C, all three end up in one conflict.
pub fn compute_conflicts(all_hunks: Vec<Hunk>) -> Vec<Conflict> {
    if all_hunks.is_empty() {
        return Vec::new();
    }

    // Sort hunks by base_start, then base_end
    let mut sorted = all_hunks;
    sorted.sort_by(|a, b| {
        a.base_start
            .cmp(&b.base_start)
            .then(a.base_end.cmp(&b.base_end))
    });

    // Merge overlapping hunks into groups
    let mut groups: Vec<(usize, usize, Vec<Hunk>)> = Vec::new();

    for hunk in sorted {
        if let Some(last) = groups.last_mut()
            && hunk.base_start <= last.1
        {
            last.1 = cmp::max(last.1, hunk.base_end);
            last.2.push(hunk);
            continue;
        }
        let end = hunk.base_end;
        let start = hunk.base_start;
        groups.push((start, end, vec![hunk]));
    }

    // Only keep groups that are genuine conflicts:
    // - 2+ distinct sources (branch OR user) involved. branch is the source
    //   for committed content; user is the source for live overlays, so two
    //   users on the same branch with divergent content DO count as a
    //   conflict between themselves.
    // - at least 2 hunks that differ in what they changed (not all identical edits)
    groups
        .into_iter()
        .filter_map(|(start, end, hunks)| {
            let mut sources = HashSet::new();
            for h in &hunks {
                sources.insert((h.branch.clone(), h.user_id));
            }
            if sources.len() < 2 {
                return None;
            }

            let first_signature = hunk_signature(&hunks[0]);
            let all_same = hunks
                .iter()
                .skip(1)
                .all(|h| hunk_signature(h) == first_signature);
            if all_same {
                return None;
            }

            Some(Conflict {
                base_start: start,
                base_end: end,
                hunks,
            })
        })
        .collect()
}

/// Produces a comparable fingerprint of what a hunk actually changes.
/// Two hunks that delete/insert the exact same lines relative to base
/// will have the same signature, meaning they're not a real conflict.
fn hunk_signature(hunk: &Hunk) -> (usize, usize, &[String]) {
    (hunk.base_start, hunk.base_end, &hunk.content)
}
