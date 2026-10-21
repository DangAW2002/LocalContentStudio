import type { GraphNode, GraphEdge, GraphNodeStatus, GraphEdgeRole } from "../../../api/types";

export interface LayoutedNode {
  id: string;
  kind: string;
  title: string;
  status: GraphNodeStatus;
  pinned: boolean;
  notes: string[];
  x: number;
  y: number;
  depth: number;
}

export interface LayoutedEdge {
  id: string;
  source: string;
  target: string;
  role: GraphEdgeRole;
}

export interface LayoutResult {
  nodes: LayoutedNode[];
  edges: LayoutedEdge[];
  width: number;
  height: number;
}

const NODE_WIDTH = 200;
const NODE_HEIGHT = 70;
const LAYER_GAP_X = 72;
const NODE_GAP_Y = 40;
const PADDING = 60;

/**
 * Strip prefix "artifact:" for consistent matching against node ids.
 */
export function normalizeRef(ref: string): string {
  if (ref.startsWith("artifact:")) {
    return ref.slice("artifact:".length);
  }
  return ref;
}

/**
 * Computes deterministic layered layout by topological depth.
 * Sources (depth 0) on the left, downstream consumers on the right.
 */
export function computeGraphLayout(
  nodes: GraphNode[],
  edges: GraphEdge[]
): LayoutResult {
  if (nodes.length === 0) {
    return { nodes: [], edges: [], width: 0, height: 0 };
  }

  const nodeMap = new Map<string, GraphNode>();
  for (const n of nodes) {
    nodeMap.set(n.id, n);
    nodeMap.set(normalizeRef(n.id), n);
  }

  // Build dependency adjacency: dependency -> consumers
  // An edge `e.from -> e.to` means `e.from` (consumer) depends on `e.to` (dependency).
  // So `e.to` is upstream (lower depth) and `e.from` is downstream (higher depth).
  const inDegree = new Map<string, number>();
  const dependents = new Map<string, Set<string>>();

  for (const n of nodes) {
    inDegree.set(n.id, 0);
    dependents.set(n.id, new Set());
  }

  for (const e of edges) {
    const fromId = nId(e.from, nodeMap);
    const toId = nId(e.to, nodeMap);
    if (fromId && toId && fromId !== toId) {
      if (!dependents.get(toId)!.has(fromId)) {
        dependents.get(toId)!.add(fromId);
        inDegree.set(fromId, (inDegree.get(fromId) || 0) + 1);
      }
    }
  }

  // Calculate layer / depth for each node
  const depths = new Map<string, number>();
  // Start with nodes having in-degree 0 (pure sources)
  const queue: string[] = [];
  for (const n of nodes) {
    if ((inDegree.get(n.id) || 0) === 0) {
      depths.set(n.id, 0);
      queue.push(n.id);
    }
  }

  // In case of cycles or disconnected components, fallback
  let visited = 0;
  while (queue.length > 0) {
    const curr = queue.shift()!;
    visited++;
    const currDepth = depths.get(curr) || 0;
    const deps = dependents.get(curr);
    if (deps) {
      for (const next of deps) {
        const nextDepth = Math.max(depths.get(next) ?? 0, currDepth + 1);
        depths.set(next, nextDepth);
        const deg = (inDegree.get(next) || 1) - 1;
        inDegree.set(next, deg);
        if (deg === 0) {
          queue.push(next);
        }
      }
    }
  }

  // If any unvisited remaining (e.g. cycle component), assign depth 0
  for (const n of nodes) {
    if (!depths.has(n.id)) {
      depths.set(n.id, 0);
    }
  }

  // Group nodes by depth
  const layers = new Map<number, GraphNode[]>();
  for (const n of nodes) {
    const d = depths.get(n.id) || 0;
    if (!layers.has(d)) {
      layers.set(d, []);
    }
    layers.get(d)!.push(n);
  }

  // Sort nodes in each layer deterministically by id
  const sortedLayers = Array.from(layers.keys()).sort((a, b) => a - b);
  for (const layerIndex of sortedLayers) {
    layers.get(layerIndex)!.sort((a, b) => a.id.localeCompare(b.id));
  }

  // Compute (x, y) coordinates
  const layoutedNodes: LayoutedNode[] = [];
  let maxX = 0;
  let maxY = 0;

  for (const layerIndex of sortedLayers) {
    const layerNodes = layers.get(layerIndex)!;
    const x = PADDING + layerIndex * (NODE_WIDTH + LAYER_GAP_X);

    layerNodes.forEach((node, nodeIndex) => {
      const y = PADDING + nodeIndex * (NODE_HEIGHT + NODE_GAP_Y);
      layoutedNodes.push({
        ...node,
        x,
        y,
        depth: layerIndex,
      });
      if (x + NODE_WIDTH > maxX) maxX = x + NODE_WIDTH;
      if (y + NODE_HEIGHT > maxY) maxY = y + NODE_HEIGHT;
    });
  }

  // Format edges with unique ids
  const layoutedEdges: LayoutedEdge[] = edges.map((e, idx) => ({
    id: `e-${e.from}-${e.to}-${idx}`,
    source: nId(e.to, nodeMap) || e.to,
    target: nId(e.from, nodeMap) || e.from,
    role: e.role,
  }));

  return {
    nodes: layoutedNodes,
    edges: layoutedEdges,
    width: maxX + PADDING,
    height: maxY + PADDING,
  };
}

function nId(ref: string, map: Map<string, GraphNode>): string | null {
  const norm = normalizeRef(ref);
  if (map.has(norm)) {
    return map.get(norm)!.id;
  }
  if (map.has(ref)) {
    return map.get(ref)!.id;
  }
  return null;
}

/**
 * Returns human-readable Vietnamese label for node status.
 */
export function getStatusLabel(status: GraphNodeStatus): string {
  switch (status) {
    case "moi":
      return "Mới";
    case "loi_thoi":
      return "Lỗi thời";
    case "chua_render":
      return "Chưa render";
    case "loi":
      return "Lỗi";
    default:
      return status;
  }
}

/**
 * Returns semantic design-system color tokens for node status.
 */
export function getStatusStyles(status: GraphNodeStatus): {
  color: string;
  bg: string;
  border: string;
} {
  switch (status) {
    case "moi":
      return {
        color: "var(--cs-success)",
        bg: "var(--cs-status-success-bg)",
        border: "var(--cs-success)",
      };
    case "loi_thoi":
      return {
        color: "var(--cs-warning)",
        bg: "var(--cs-status-warning-bg)",
        border: "var(--cs-warning)",
      };
    case "chua_render":
      return {
        color: "var(--cs-fg-muted)",
        bg: "var(--cs-status-neutral-bg)",
        border: "var(--cs-border-subtle)",
      };
    case "loi":
      return {
        color: "var(--cs-danger)",
        bg: "var(--cs-status-danger-bg)",
        border: "var(--cs-danger)",
      };
  }
}

/**
 * Returns stroke style props for edge role.
 */
export function getEdgeRoleStyle(role: GraphEdgeRole): {
  strokeDasharray?: string;
  stroke: string;
} {
  switch (role) {
    case "uses":
      return { stroke: "var(--cs-fg-secondary)" };
    case "reference":
      return { strokeDasharray: "5 5", stroke: "var(--cs-accent)" };
    case "variant_of":
      return { strokeDasharray: "2 3", stroke: "var(--cs-fg-muted)" };
  }
}
