//! Graph editing operations (linking, unlinking, pinning, notes, variants, downstream updates).
//! Plan 0009 §4, ADR-011 §1, §2, §4, §5.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use studio_workspace::{CheckpointService, Project, Workspace, WorkspaceDb};

use crate::artifact::{Artifact, ArtifactInput, InputRole};
use crate::error::{ArtifactError, Result};
use crate::graph::{detect_cycle, normalize_ref, topo_sort};
use crate::render::{effective_renderer, render_options_for, render_with};

/// Helper to synchronize artifact edges into SQLite `artifact_edges` table for a specific project.
pub fn reindex_project_edges(ws: &Workspace, project: &Project) -> Result<()> {
    let db = WorkspaceDb::open(ws)?;
    let artifacts = Artifact::list(project)?;

    // We only update edges for this project
    db.conn().execute(
        "DELETE FROM artifact_edges WHERE project = ?1",
        rusqlite::params![project.slug()],
    )?;

    for art in artifacts {
        for inp in &art.inputs {
            db.conn().execute(
                "INSERT OR REPLACE INTO artifact_edges (project, from_id, to_ref, role) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    project.slug(),
                    art.id,
                    inp.reference,
                    inp.role.as_str(),
                ],
            )?;
        }
    }

    Ok(())
}

/// Link two nodes in the content graph: `from_artifact_id` depends on `to_ref` with a given `role`.
/// 1. Verifies non-existence of cycle before applying.
/// 2. Creates a checkpoint.
/// 3. Updates `artifact.yaml` of `from_artifact_id`.
/// 4. Reindexes SQLite edges.
pub fn graph_link(
    ws: &Workspace,
    project: &Project,
    from_id: &str,
    to_ref: &str,
    role: InputRole,
) -> Result<()> {
    let mut art = Artifact::load(project, from_id)?;
    let to_norm = normalize_ref(to_ref);

    // Self-loop check
    if from_id == to_norm {
        return Err(ArtifactError::ValidationFailed(format!(
            "Không thể tạo liên kết vòng lặp: artifact \"{from_id}\" không thể tự liên kết với chính nó."
        )));
    }

    // Check if edge already exists with exact same role
    if art.inputs.iter().any(|i| {
        (i.reference == to_ref || normalize_ref(&i.reference) == to_norm) && i.role == role
    }) {
        return Ok(()); // Already linked
    }

    // Build temporary edge set to detect cycles
    let artifacts = Artifact::list(project)?;
    let mut nodes: Vec<String> = artifacts.iter().map(|a| a.id.clone()).collect();
    if !nodes.contains(&to_norm) && !to_ref.starts_with("asset:") {
        nodes.push(to_norm.clone());
    }

    let mut simple_edges: Vec<(String, String)> = Vec::new();
    for a in &artifacts {
        for inp in &a.inputs {
            let dep = normalize_ref(&inp.reference);
            simple_edges.push((a.id.clone(), dep));
        }
    }
    // Add the proposed edge
    simple_edges.push((from_id.to_string(), to_norm.clone()));

    if let Some(cycle) = detect_cycle(&nodes, &simple_edges) {
        return Err(ArtifactError::ValidationFailed(format!(
            "Không thể tạo liên kết vì sẽ gây ra chu trình phụ thuộc (vòng): {}.",
            cycle.join(" -> ")
        )));
    }

    // Checkpoint before file write
    CheckpointService::create(
        ws,
        &format!("Nối cạnh {} -> {} ({})", from_id, to_ref, role.as_str()),
        None,
    )?;

    // Update artifact.inputs
    // Remove existing edge to the same target if any
    art.inputs
        .retain(|i| i.reference != to_ref && normalize_ref(&i.reference) != to_norm);

    // If role is uses and it was string or simple ref, preserve formatting or add as object
    art.inputs.push(ArtifactInput {
        reference: to_ref.to_string(),
        role,
        was_string: false,
    });

    art.save(project)?;
    reindex_project_edges(ws, project)?;

    Ok(())
}

/// Unlink a directed dependency: remove `to_ref` from `from_artifact_id.inputs`.
pub fn graph_unlink(ws: &Workspace, project: &Project, from_id: &str, to_ref: &str) -> Result<()> {
    let mut art = Artifact::load(project, from_id)?;
    let to_norm = normalize_ref(to_ref);

    let initial_len = art.inputs.len();
    art.inputs
        .retain(|i| i.reference != to_ref && normalize_ref(&i.reference) != to_norm);

    if art.inputs.len() == initial_len {
        return Ok(()); // Nothing was removed
    }

    CheckpointService::create(ws, &format!("Xoá liên kết {} -> {}", from_id, to_ref), None)?;

    art.save(project)?;
    reindex_project_edges(ws, project)?;

    Ok(())
}

/// Set pinned status of an artifact.
pub fn graph_set_pinned(ws: &Workspace, project: &Project, id: &str, pinned: bool) -> Result<()> {
    let mut art = Artifact::load(project, id)?;
    if art.pinned == pinned {
        return Ok(());
    }

    let action = if pinned { "Khóa" } else { "Mở khóa" };
    CheckpointService::create(ws, &format!("{action} artifact {id}"), None)?;

    art.pinned = pinned;
    art.save(project)?;

    Ok(())
}

/// Set user notes of an artifact.
pub fn graph_set_notes(
    ws: &Workspace,
    project: &Project,
    id: &str,
    notes: Vec<String>,
) -> Result<()> {
    let mut art = Artifact::load(project, id)?;
    if art.notes == notes {
        return Ok(());
    }

    CheckpointService::create(ws, &format!("Cập nhật ghi chú artifact {id}"), None)?;

    art.notes = notes;
    art.save(project)?;

    Ok(())
}

/// Create a variant of an existing artifact.
/// - Creates a new artifact directory and default template for its kind.
/// - Appends `{ ref: artifact:<source_id>, role: variant_of }` to inputs.
/// - Checkpoints before creation.
pub fn graph_create_variant(
    ws: &Workspace,
    project: &Project,
    source_id: &str,
    new_id: &str,
    title: Option<String>,
) -> Result<Artifact> {
    let source_art = Artifact::load(project, source_id)?;

    let variant_title = title.unwrap_or_else(|| format!("{} (Biến thể)", source_art.title));

    CheckpointService::create(
        ws,
        &format!("Tạo biến thể {} từ {}", new_id, source_id),
        None,
    )?;

    let mut variant = Artifact::create(
        project,
        new_id,
        &source_art.kind,
        variant_title,
        source_art.brand.clone(),
        source_art.channels.clone(),
        Some(source_art.params.clone()),
    )?;

    variant.inputs.push(ArtifactInput {
        reference: format!("artifact:{}", source_id),
        role: InputRole::VariantOf,
        was_string: false,
    });

    variant.save(project)?;
    reindex_project_edges(ws, project)?;

    Ok(variant)
}

/// Create a reference profile from an artifact or asset node.
/// Checkpoint before creation.
pub fn graph_make_reference(
    ws: &Workspace,
    project: &Project,
    node_id: &str,
    new_id: Option<String>,
    title: Option<String>,
) -> Result<Artifact> {
    let clean_id = crate::graph::normalize_ref(node_id);
    let ref_id = new_id.unwrap_or_else(|| {
        if clean_id.starts_with("ref-") {
            clean_id.clone()
        } else {
            format!("ref-{}", clean_id.replace(['/', '\\'], "-"))
        }
    });

    CheckpointService::create(ws, &format!("Đặt làm hồ sơ tham chiếu: {}", ref_id), None)?;

    // Check if node is an existing artifact in project
    if let Ok(source_art) = Artifact::load(project, &clean_id) {
        let ref_title = title.unwrap_or_else(|| format!("Hồ sơ {}", source_art.title));
        let art = Artifact::create(
            project,
            &ref_id,
            "reference",
            ref_title,
            source_art.brand.clone(),
            source_art.channels.clone(),
            Some(source_art.params.clone()),
        )?;

        // Copy any images from source artifact dir
        let src_dir = project.artifacts_dir().join(&clean_id);
        let dest_dir = project.artifacts_dir().join(&ref_id);
        if src_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&src_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let ext = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if ["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) {
                        if let Some(name) = path.file_name() {
                            let _ = std::fs::copy(&path, dest_dir.join(name));
                        }
                    }
                }
            }
        }
        reindex_project_edges(ws, project)?;
        Ok(art)
    } else {
        // Node is an asset or path
        let ref_title = title.unwrap_or_else(|| format!("Hồ sơ {}", clean_id));
        let art = Artifact::create(project, &ref_id, "reference", ref_title, None, vec![], None)?;

        // If node_id is an image in workspace, copy it to the reference folder
        if let Ok(asset_path) = ws.scoped_path(std::path::Path::new(&clean_id)) {
            if asset_path.is_file() {
                let dest_dir = project.artifacts_dir().join(&ref_id);
                if let Some(name) = asset_path.file_name() {
                    let _ = std::fs::copy(&asset_path, dest_dir.join(name));
                }
            }
        }
        reindex_project_edges(ws, project)?;
        Ok(art)
    }
}

/// Downstream execution step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownstreamStep {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub renderer: Option<String>,
    pub paid: bool,
    pub estimated_cost_usd: Option<f64>,
    pub pinned: bool,
    pub skipped: bool,
    pub skip_reason: Option<String>,
}

/// Downstream update plan (dry run).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownstreamPlan {
    pub target_ref: String,
    pub steps: Vec<DownstreamStep>,
    pub free_steps_count: usize,
    pub paid_steps_count: usize,
    pub total_estimated_cost_usd: f64,
}

/// Compute downstream update plan in topological order.
pub fn graph_update_downstream_plan(
    _ws: &Workspace,
    project: &Project,
    target_ref: &str,
) -> Result<DownstreamPlan> {
    let artifacts = Artifact::list(project)?;
    let norm_target = normalize_ref(target_ref);

    let mut art_map: BTreeMap<String, &Artifact> = BTreeMap::new();
    for a in &artifacts {
        art_map.insert(a.id.clone(), a);
    }

    // Find downstream reachability along `uses` and `variant_of`
    let mut downstream: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(norm_target.clone());

    while let Some(curr) = queue.pop_front() {
        for a in &artifacts {
            if !downstream.contains(&a.id) {
                for inp in &a.inputs {
                    if normalize_ref(&inp.reference) == curr && inp.role.propagates_staleness() {
                        downstream.insert(a.id.clone());
                        queue.push_back(a.id.clone());
                        break;
                    }
                }
            }
        }
    }

    // Topological order
    let sub_nodes: Vec<String> = downstream.iter().cloned().collect();
    let mut sub_edges: Vec<(String, String)> = Vec::new();
    for a in &artifacts {
        for inp in &a.inputs {
            let dep = normalize_ref(&inp.reference);
            if downstream.contains(&a.id)
                && downstream.contains(&dep)
                && inp.role.propagates_staleness()
            {
                sub_edges.push((a.id.clone(), dep));
            }
        }
    }

    let ordered = topo_sort(&sub_nodes, &sub_edges).unwrap_or(sub_nodes);

    let mut steps = Vec::new();
    let mut free_cnt = 0;
    let mut paid_cnt = 0;
    let mut total_cost = 0.0;

    for id in ordered {
        if let Some(art) = art_map.get(&id) {
            let renderer = effective_renderer(art);
            let is_generation = art.kind == "generation";
            let paid = is_generation;
            let cost = if paid { None } else { Some(0.0) };

            let mut skipped = false;
            let mut skip_reason = None;

            if art.pinned {
                skipped = true;
                skip_reason = Some("Đã khóa (pinned)".to_string());
            }

            if !skipped {
                if paid {
                    paid_cnt += 1;
                    if let Some(c) = cost {
                        total_cost += c;
                    }
                } else {
                    free_cnt += 1;
                }
            }

            steps.push(DownstreamStep {
                id: art.id.clone(),
                kind: art.kind.clone(),
                title: art.title.clone(),
                renderer: Some(renderer),
                paid,
                estimated_cost_usd: cost,
                pinned: art.pinned,
                skipped,
                skip_reason,
            });
        }
    }

    Ok(DownstreamPlan {
        target_ref: target_ref.to_string(),
        steps,
        free_steps_count: free_cnt,
        paid_steps_count: paid_cnt,
        total_estimated_cost_usd: total_cost,
    })
}

/// Run downstream update for an artifact: executes all unpinned FREE renders in topological order.
/// Returns the updated plan showing what was executed and what remains (paid or skipped).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownstreamRunResult {
    pub executed_ids: Vec<String>,
    pub remaining_paid: Vec<DownstreamStep>,
    pub skipped_pinned: Vec<String>,
    pub errors: Vec<String>,
}

pub async fn graph_update_downstream_run(
    ws: &Workspace,
    project: &Project,
    target_ref: &str,
    scheduler: &studio_jobs::Scheduler,
    run_paid: bool,
) -> Result<DownstreamRunResult> {
    let plan = graph_update_downstream_plan(ws, project, target_ref)?;

    let mut executed_ids = Vec::new();
    let mut remaining_paid = Vec::new();
    let mut skipped_pinned = Vec::new();
    let mut errors = Vec::new();

    for step in plan.steps {
        if step.pinned {
            skipped_pinned.push(step.id);
            continue;
        }

        if step.paid && !run_paid {
            remaining_paid.push(step);
            continue;
        }

        // Execute render
        let art = match Artifact::load(project, &step.id) {
            Ok(a) => a,
            Err(e) => {
                errors.push(format!("{}: Không tải được: {e}", step.id));
                continue;
            }
        };

        let options = render_options_for(ws, &art);
        match render_with(ws, project, &art, scheduler, &options).await {
            Ok(_) => {
                executed_ids.push(step.id);
            }
            Err(e) => {
                errors.push(format!("{}: Render lỗi: {e}", step.id));
            }
        }
    }

    Ok(DownstreamRunResult {
        executed_ids,
        remaining_paid,
        skipped_pinned,
        errors,
    })
}

/// Post-turn lock check: the pinned artifacts (any project) whose files differ from the
/// pre-turn checkpoint. Compares against the checkpoint without creating a new one, and does
/// nothing when no artifact is pinned.
pub fn check_pinned_modifications_after_turn(
    ws: &Workspace,
    pre_turn_cp_id: &str,
) -> Result<Vec<String>> {
    let mut pinned: Vec<(String, String)> = Vec::new(); // (prefix, artifact id)
    if let Ok(projects) = ws.list_projects() {
        for proj in projects {
            for art in Artifact::list(&proj).unwrap_or_default() {
                if art.pinned {
                    pinned.push((
                        format!("projects/{}/artifacts/{}/", proj.slug(), art.id),
                        art.id,
                    ));
                }
            }
        }
    }
    if pinned.is_empty() {
        return Ok(Vec::new());
    }
    let prefixes: Vec<String> = pinned.iter().map(|(p, _)| p.clone()).collect();
    let changed = CheckpointService::changed_under(ws, pre_turn_cp_id, &prefixes)?;
    let ids: BTreeSet<String> = pinned
        .into_iter()
        .filter(|(p, _)| changed.contains(p))
        .map(|(_, id)| id)
        .collect();
    Ok(ids.into_iter().collect())
}

/// Restore only the files of one artifact to their state in `checkpoint_id` (works even when the
/// turn deleted its `artifact.yaml`). A checkpoint is taken first so this can be undone too.
pub fn restore_artifact_files_from_checkpoint(
    ws: &Workspace,
    project_slug: &str,
    artifact_id: &str,
    checkpoint_id: &str,
) -> Result<()> {
    let proj = Project::open(ws, project_slug)?;
    CheckpointService::create(
        ws,
        &format!("Trước khi hoàn tác artifact khoá {artifact_id}"),
        None,
    )?;
    let prefix = format!("projects/{project_slug}/artifacts/{artifact_id}/");
    CheckpointService::restore_under(ws, checkpoint_id, &prefix)?;
    reindex_project_edges(ws, &proj)?;
    Ok(())
}
