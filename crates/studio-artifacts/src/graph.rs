//! Content dependency graph module (pure functions + deterministic topological sort + staleness propagation).
//! Plan 0009 §4, ADR-011 §1-§2.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use studio_workspace::{Project, Workspace};

use crate::artifact::{Artifact, InputRole};
use crate::cache::{compute_input_hash, latest_render_on_disk};
use crate::error::Result;

/// Status of an artifact node in the content graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    /// Newly rendered and up to date.
    Moi,
    /// Outdated compared to latest input hash or upstream changes.
    LoiThoi,
    /// Never rendered yet.
    ChuaRender,
    /// Erroneous state or failed check.
    Loi,
}

impl NodeStatus {
    /// Vietnamese string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Moi => "moi",
            Self::LoiThoi => "loi_thoi",
            Self::ChuaRender => "chua_render",
            Self::Loi => "loi",
        }
    }
}

/// A node in the content graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphNode {
    /// Artifact ID or asset ref path (e.g. "thumb-01", "asset:lib/logo.png")
    pub id: String,
    /// Artifact kind: "design", "motion", "timeline", "text", "generation", "audio", or "asset".
    pub kind: String,
    /// Display title.
    pub title: String,
    /// Node status: moi, loi_thoi, chua_render, loi.
    pub status: NodeStatus,
    /// Whether user locked the node from agent edits / auto-regeneration.
    pub pinned: bool,
    /// User notes attached to this artifact node.
    pub notes: Vec<String>,
}

/// A directed edge in the content graph: `from -> to` with a semantic role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GraphEdge {
    /// Source artifact ID (the consumer).
    pub from: String,
    /// Target reference (the dependency, e.g. "script-01" or "artifact:script-01" or "asset:...").
    pub to: String,
    /// Semantic role: uses, reference, variant_of.
    pub role: InputRole,
}

/// Content dependency graph for a project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ContentGraph {
    /// Nodes in the graph.
    pub nodes: Vec<GraphNode>,
    /// Edges in the graph.
    pub edges: Vec<GraphEdge>,
}

/// Impact item describing a downstream node affected by a change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImpactNode {
    /// Artifact identifier.
    pub id: String,
    /// Artifact kind.
    pub kind: String,
    /// Artifact title.
    pub title: String,
    /// Renderer tool id if known.
    pub renderer: Option<String>,
    /// Whether the generation / capability has paid side effects.
    pub paid: Option<bool>,
    /// Estimated cost in USD if known.
    pub estimated_cost_usd: Option<f64>,
}

/// Normalize reference string to artifact id if it begins with "artifact:".
pub fn normalize_ref(r: &str) -> String {
    if let Some(stripped) = r.strip_prefix("artifact:") {
        stripped.to_string()
    } else {
        r.to_string()
    }
}

/// Detect cycle in a directed graph of nodes and edges.
/// If a cycle is detected, returns `Some(Vec<String>)` containing the cycle path.
pub fn detect_cycle(nodes: &[String], edges: &[(String, String)]) -> Option<Vec<String>> {
    let mut adj: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for n in nodes {
        adj.entry(n.as_str()).or_default();
    }
    for (from, to) in edges {
        adj.entry(from.as_str()).or_default().push(to.as_str());
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Unvisited,
        Visiting,
        Visited,
    }

    let mut state: BTreeMap<&str, State> = BTreeMap::new();
    for n in nodes {
        state.insert(n.as_str(), State::Unvisited);
    }

    let mut path: Vec<String> = Vec::new();

    fn dfs<'a>(
        u: &'a str,
        adj: &BTreeMap<&'a str, Vec<&'a str>>,
        state: &mut BTreeMap<&'a str, State>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        state.insert(u, State::Visiting);
        path.push(u.to_string());

        if let Some(neighbors) = adj.get(u) {
            for &v in neighbors {
                match state.get(v).copied().unwrap_or(State::Unvisited) {
                    State::Visiting => {
                        // Cycle detected
                        let mut cycle = Vec::new();
                        let mut in_cycle = false;
                        for p in path.iter() {
                            if p == v {
                                in_cycle = true;
                            }
                            if in_cycle {
                                cycle.push(p.clone());
                            }
                        }
                        cycle.push(v.to_string());
                        return Some(cycle);
                    }
                    State::Unvisited => {
                        if let Some(cycle) = dfs(v, adj, state, path) {
                            return Some(cycle);
                        }
                    }
                    State::Visited => {}
                }
            }
        }

        path.pop();
        state.insert(u, State::Visited);
        None
    }

    for n in nodes {
        if state.get(n.as_str()) == Some(&State::Unvisited) {
            if let Some(cycle) = dfs(n.as_str(), &adj, &mut state, &mut path) {
                return Some(cycle);
            }
        }
    }

    None
}

/// Deterministic topological order for the nodes.
/// Direction: dependencies come first (sources before consumers).
/// If `A uses B`, edge in `edges` is `(A, B)`. In topo order: `B` appears before `A`.
/// Ties are broken deterministically by node id alphabetical order.
pub fn topo_sort(nodes: &[String], edges: &[(String, String)]) -> Option<Vec<String>> {
    // In-degree for consumer: how many dependencies does `u` wait on?
    // Dependency B has outgoing directed edge to consumer A in dependency resolution order.
    let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for n in nodes {
        in_degree.insert(n.clone(), 0);
        dependents.insert(n.clone(), BTreeSet::new());
    }

    for (from, to) in edges {
        if in_degree.contains_key(from) && in_degree.contains_key(to) {
            // `from` depends on `to`. So `to` must come before `from`.
            // When `to` is resolved, `from`'s in-degree decreases.
            if dependents
                .entry(to.clone())
                .or_default()
                .insert(from.clone())
            {
                *in_degree.entry(from.clone()).or_default() += 1;
            }
        }
    }

    // Min-priority queue or sorted list for deterministic Kahn's algorithm
    let mut ready: BTreeSet<String> = BTreeSet::new();
    for (n, deg) in &in_degree {
        if *deg == 0 {
            ready.insert(n.clone());
        }
    }

    let mut result = Vec::new();
    while let Some(first) = ready.iter().next().cloned() {
        ready.remove(&first);
        result.push(first.clone());

        if let Some(deps) = dependents.get(&first) {
            for dep in deps {
                let deg = in_degree.get_mut(dep).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    ready.insert(dep.clone());
                }
            }
        }
    }

    if result.len() == nodes.len() {
        Some(result)
    } else {
        None
    }
}

/// Compute graph staleness and build the full `ContentGraph`.
/// An artifact is:
/// - `ChuaRender` if it has never been rendered.
/// - `LoiThoi` if its current input hash differs from its latest render on disk,
///   OR if an upstream dependency connected by `uses` or `variant_of` is `LoiThoi`.
/// - `Moi` if current input hash matches latest render on disk.
/// - `Loi` if any error or failed checks exist.
pub fn build_project_graph(
    workspace: &Workspace,
    project: &Project,
    artifacts: &[Artifact],
) -> Result<ContentGraph> {
    let mut nodes_map: BTreeMap<String, GraphNode> = BTreeMap::new();
    let mut raw_edges: Vec<GraphEdge> = Vec::new();

    let default_brand_version = "v1";

    // 1. First pass: compute local staleness for each artifact
    for art in artifacts {
        let renderer_id = art.renderer.as_deref().unwrap_or("render.none@1");
        let brand_version = art.brand.as_deref().unwrap_or(default_brand_version);

        let status = match latest_render_on_disk(workspace, &art.id) {
            Ok(Some((disk_hash, _files))) => {
                // Compute current input hash
                match compute_input_hash(workspace, project, art, brand_version, renderer_id) {
                    Ok(current_hash) => {
                        if current_hash == disk_hash {
                            NodeStatus::Moi
                        } else {
                            NodeStatus::LoiThoi
                        }
                    }
                    Err(_) => NodeStatus::Loi,
                }
            }
            Ok(None) => NodeStatus::ChuaRender,
            Err(_) => NodeStatus::Loi,
        };

        nodes_map.insert(
            art.id.clone(),
            GraphNode {
                id: art.id.clone(),
                kind: art.kind.clone(),
                title: art.title.clone(),
                status,
                pinned: art.pinned,
                notes: art.notes.clone(),
            },
        );

        for inp in &art.inputs {
            let target_ref = inp.reference.clone();
            raw_edges.push(GraphEdge {
                from: art.id.clone(),
                to: target_ref,
                role: inp.role,
            });
        }
    }

    // 2. Add external/asset nodes for references that are not artifacts
    for edge in &raw_edges {
        let norm = normalize_ref(&edge.to);
        if !nodes_map.contains_key(&norm) && !nodes_map.contains_key(&edge.to) {
            // It is an external asset or file path
            let display_id = edge.to.clone();
            nodes_map.insert(
                display_id.clone(),
                GraphNode {
                    id: display_id.clone(),
                    kind: "asset".to_string(),
                    title: display_id.clone(),
                    status: NodeStatus::Moi,
                    pinned: false,
                    notes: Vec::new(),
                },
            );
        }
    }

    // 3. Propagate staleness downstream along `uses` and `variant_of`
    // Extract node list and edge pairs for topo sort
    let all_node_ids: Vec<String> = nodes_map.keys().cloned().collect();
    let simple_edges: Vec<(String, String)> = raw_edges
        .iter()
        .map(|e| (e.from.clone(), normalize_ref(&e.to)))
        .collect();

    // In dependency order: dependencies come first, consumers after
    if let Some(order) = topo_sort(&all_node_ids, &simple_edges) {
        let mut stale_set: BTreeSet<String> = BTreeSet::new();

        // Seed with nodes that are already LoiThoi
        for (id, node) in &nodes_map {
            if node.status == NodeStatus::LoiThoi {
                stale_set.insert(id.clone());
            }
        }

        // Forward propagation along topo order
        // (from dependency to consumer)
        for dep_id in order {
            if stale_set.contains(&dep_id) {
                // Find all consumers that depend on this dep with role uses or variant_of
                for edge in &raw_edges {
                    let target = normalize_ref(&edge.to);
                    if (target == dep_id || edge.to == dep_id) && edge.role.propagates_staleness() {
                        let consumer_id = &edge.from;
                        stale_set.insert(consumer_id.clone());
                        if let Some(consumer_node) = nodes_map.get_mut(consumer_id) {
                            if consumer_node.status != NodeStatus::ChuaRender {
                                consumer_node.status = NodeStatus::LoiThoi;
                            }
                        }
                    }
                }
            }
        }
    }

    let mut nodes: Vec<GraphNode> = nodes_map.into_values().collect();
    nodes.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(ContentGraph {
        nodes,
        edges: raw_edges,
    })
}

/// Calculate downstream impact of changing an artifact or ref.
/// Returns downstream nodes in topological order, along with renderer and whether paid.
pub fn calculate_impact(
    _workspace: &Workspace,
    _project: &Project,
    artifacts: &[Artifact],
    target_ref: &str,
) -> Vec<ImpactNode> {
    let norm_target = normalize_ref(target_ref);

    // Build artifact lookup
    let mut art_map: BTreeMap<String, &Artifact> = BTreeMap::new();
    for a in artifacts {
        art_map.insert(a.id.clone(), a);
    }

    // Build edges (consumer -> dependency)
    let mut simple_edges: Vec<(String, String)> = Vec::new();
    let mut nodes_list: Vec<String> = art_map.keys().cloned().collect();
    if !nodes_list.contains(&norm_target) {
        nodes_list.push(norm_target.clone());
    }

    for a in artifacts {
        for inp in &a.inputs {
            let dep = normalize_ref(&inp.reference);
            simple_edges.push((a.id.clone(), dep));
        }
    }

    // Downstream reachability via BFS/DFS (only follows edges where target is dependency)
    let mut downstream: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(norm_target.clone());

    while let Some(curr) = queue.pop_front() {
        for a in artifacts {
            if !downstream.contains(&a.id) {
                for inp in &a.inputs {
                    if normalize_ref(&inp.reference) == curr {
                        downstream.insert(a.id.clone());
                        queue.push_back(a.id.clone());
                        break;
                    }
                }
            }
        }
    }

    // Sort downstream in topological order
    let sub_nodes: Vec<String> = downstream.iter().cloned().collect();
    let sub_edges: Vec<(String, String)> = simple_edges
        .into_iter()
        .filter(|(from, to)| downstream.contains(from) && downstream.contains(to))
        .collect();

    let ordered = topo_sort(&sub_nodes, &sub_edges).unwrap_or(sub_nodes);

    let mut results = Vec::new();
    for id in ordered {
        if let Some(art) = art_map.get(&id) {
            let (paid, cost) = match art.kind.as_str() {
                "generation" => (Some(true), None),
                "design" | "timeline" | "motion" | "text" => (Some(false), Some(0.0)),
                _ => (None, None),
            };

            results.push(ImpactNode {
                id: art.id.clone(),
                kind: art.kind.clone(),
                title: art.title.clone(),
                renderer: art.renderer.clone(),
                paid,
                estimated_cost_usd: cost,
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_detection() {
        let nodes = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let edges = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "C".to_string()),
            ("C".to_string(), "A".to_string()),
        ];
        let cycle = detect_cycle(&nodes, &edges);
        assert!(cycle.is_some());
        let c = cycle.unwrap();
        assert!(c.contains(&"A".to_string()));

        let acyclic_edges = vec![
            ("A".to_string(), "B".to_string()),
            ("B".to_string(), "C".to_string()),
        ];
        assert!(detect_cycle(&nodes, &acyclic_edges).is_none());
    }

    #[test]
    fn test_deterministic_topo_order() {
        let nodes = vec![
            "thumb".to_string(),
            "timeline".to_string(),
            "script".to_string(),
            "voice".to_string(),
        ];
        // thumb uses script
        // timeline uses script, voice
        // voice uses script
        let edges = vec![
            ("thumb".to_string(), "script".to_string()),
            ("timeline".to_string(), "script".to_string()),
            ("timeline".to_string(), "voice".to_string()),
            ("voice".to_string(), "script".to_string()),
        ];

        let sorted = topo_sort(&nodes, &edges).expect("Acyclic graph must sort");
        // script has no dependencies -> must come before voice, thumb, timeline
        let idx_script = sorted.iter().position(|x| x == "script").unwrap();
        let idx_voice = sorted.iter().position(|x| x == "voice").unwrap();
        let idx_timeline = sorted.iter().position(|x| x == "timeline").unwrap();
        let idx_thumb = sorted.iter().position(|x| x == "thumb").unwrap();

        assert!(idx_script < idx_voice);
        assert!(idx_script < idx_thumb);
        assert!(idx_voice < idx_timeline);
    }

    #[test]
    fn test_staleness_propagation_uses_vs_reference() {
        let temp = tempfile::tempdir().unwrap();
        let ws = Workspace::create(temp.path(), "Stale Test Workspace").unwrap();
        let proj = Project::create(&ws, "proj", "Project").unwrap();

        // 1. Script artifact (source)
        let script =
            Artifact::create(&proj, "script", "text", "Script", None, vec![], None).unwrap();

        // 2. Timeline artifact (uses script)
        let mut timeline = Artifact::create(
            &proj,
            "timeline",
            "timeline",
            "Timeline",
            None,
            vec![],
            None,
        )
        .unwrap();
        timeline.inputs = vec![crate::artifact::ArtifactInput::new(
            "artifact:script",
            InputRole::Uses,
        )];
        timeline.save(&proj).unwrap();

        // 3. Thumb artifact (variant_of timeline)
        let mut thumb =
            Artifact::create(&proj, "thumb", "design", "Thumbnail", None, vec![], None).unwrap();
        thumb.inputs = vec![crate::artifact::ArtifactInput::new(
            "artifact:timeline",
            InputRole::VariantOf,
        )];
        thumb.save(&proj).unwrap();

        // 4. Style profile (reference - does NOT propagate staleness)
        let style_ref =
            Artifact::create(&proj, "style", "design", "Style Guide", None, vec![], None).unwrap();

        let mut poster =
            Artifact::create(&proj, "poster", "design", "Poster", None, vec![], None).unwrap();
        poster.inputs = vec![crate::artifact::ArtifactInput::new(
            "artifact:style",
            InputRole::Reference,
        )];
        poster.save(&proj).unwrap();

        // Fake render for all artifacts so they have a baseline hash in WorkspaceDb renders table
        for art in [&script, &timeline, &thumb, &style_ref, &poster] {
            let renderer_id = art.renderer.as_deref().unwrap_or("render.none@1");
            let h = crate::cache::compute_input_hash(&ws, &proj, art, "v1", renderer_id).unwrap();
            let out_dir = proj
                .renders_dir()
                .join(&art.id)
                .join(crate::cache::hash12(&h));
            std::fs::create_dir_all(&out_dir).unwrap();
            let fake_out = out_dir.join("output.png");
            std::fs::write(&fake_out, b"123").unwrap();
            crate::cache::record_render_outputs(&ws, &art.id, &h, &[fake_out]).unwrap();
        }

        // Before changes: all should be Moi
        let graph1 = build_project_graph(
            &ws,
            &proj,
            &[
                script.clone(),
                timeline.clone(),
                thumb.clone(),
                style_ref.clone(),
                poster.clone(),
            ],
        )
        .unwrap();
        for node in &graph1.nodes {
            assert_eq!(
                node.status,
                NodeStatus::Moi,
                "Node {} should be up to date initially",
                node.id
            );
        }

        // Now modify script source -> script becomes stale (LoiThoi)
        std::fs::write(script.source_path(&proj), "Updated script text").unwrap();

        // And modify style_ref source -> style_ref becomes stale (LoiThoi)
        std::fs::write(style_ref.source_path(&proj), "Updated style guide").unwrap();

        let graph2 = build_project_graph(
            &ws,
            &proj,
            &[
                script.clone(),
                timeline.clone(),
                thumb.clone(),
                style_ref.clone(),
                poster.clone(),
            ],
        )
        .unwrap();
        let map: std::collections::BTreeMap<String, NodeStatus> =
            graph2.nodes.into_iter().map(|n| (n.id, n.status)).collect();

        // script is directly modified -> LoiThoi
        assert_eq!(map.get("script"), Some(&NodeStatus::LoiThoi));
        // timeline uses script -> LoiThoi (propagated)
        assert_eq!(map.get("timeline"), Some(&NodeStatus::LoiThoi));
        // thumb variant_of timeline -> LoiThoi (propagated)
        assert_eq!(map.get("thumb"), Some(&NodeStatus::LoiThoi));

        // style is directly modified -> LoiThoi
        assert_eq!(map.get("style"), Some(&NodeStatus::LoiThoi));
        // poster references style -> MUST REMAIN Moi (reference role does NOT propagate staleness!)
        assert_eq!(map.get("poster"), Some(&NodeStatus::Moi));
    }
}
