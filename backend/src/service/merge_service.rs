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
pub async fn calculate_live_diff(
    file_name: String,
    project_id: Uuid,
    state: web::Data<AppState>,
    base: &Path,
) -> Result<Vec<Conflict>, LGitIoError> {
    let file_path = Path::new(&file_name);

    // 1. Retrieve base content from the base branch (read_file already fetched origin)
    let base_content = git_service::read_file(base, "main", file_path)
        .await
        .unwrap_or_else(|_| String::new());

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

    // 2. Get all active remote branches
    let file_overlays = extract_overlay_file_contents(file_name.clone(), project_id, state)?;
    // <(branch_name, content)>
    let mut contents: Vec<(String, String)> = Vec::new();
    // Overlays take precedence; remove redundant branches
    file_overlays.into_iter().for_each(|f| {
        if active_branches.contains(&f.0) {
            active_branches.remove(&f.0);
        }
        // Add overlay contents
        contents.push((f.0, f.1));
    });

    // Add branch contents
    let mut branch_contents: Vec<(String, String)> = stream::iter(active_branches.iter().cloned())
        .map(|branch: String| async move {
            (
                branch.clone(),
                git_service::read_file(base, branch.as_str(), file_path)
                    .await
                    .unwrap(),
            )
        })
        .buffer_unordered(10)
        .collect::<Vec<(String, String)>>()
        .await;

    // 6. Combine all contents (overlays + branches)
    contents.append(&mut branch_contents);

    // 7. Compute diff against base for combined contents
    let diff = compute_combined_diff(base_content, contents);
    // 8. Compute merge conflicts based on each diff through overlap checks
    let conflicts = compute_conflicts(diff);

    Ok(conflicts)
}

/// Decompose a single branch's diff against base into contiguous hunks of
/// changed lines, tagged with the branch name for later overlap analysis.
pub fn compute_combined_diff(
    base_content: String,
    branch_contents: Vec<(String, String)>,
) -> Vec<Hunk> {
    let mut all_hunks: Vec<Hunk> = Vec::new();

    for (branch, content) in branch_contents {
        if content == base_content {
            println!("[diff] branch={} IDENTICAL to base, skipping", branch);
            continue;
        }
        println!(
            "[diff] branch={} differs from base (base_len={}, branch_len={})",
            branch,
            base_content.len(),
            content.len()
        );

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
    // - 2+ distinct branches involved
    // - at least 2 hunks that differ in what they changed (not all identical edits)
    groups
        .into_iter()
        .filter_map(|(start, end, hunks)| {
            let mut branches = HashSet::new();
            for h in &hunks {
                branches.insert(&h.branch);
            }
            if branches.len() < 2 {
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
/// will have the same signature — meaning they're not a real conflict.
fn hunk_signature(hunk: &Hunk) -> (usize, usize, &[String]) {
    (hunk.base_start, hunk.base_end, &hunk.content)
}
