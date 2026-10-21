import { describe, it, expect } from "vitest";
import {
  computeGraphLayout,
  getStatusLabel,
  getStatusStyles,
  getEdgeRoleStyle,
  normalizeRef,
} from "./graph-layout";
import type { GraphNode, GraphEdge } from "../../../api/types";

describe("graph-layout", () => {
  it("normalizes reference prefixes", () => {
    expect(normalizeRef("artifact:script-01")).toBe("script-01");
    expect(normalizeRef("asset:lib/logo.png")).toBe("asset:lib/logo.png");
    expect(normalizeRef("thumb")).toBe("thumb");
  });

  it("maps status labels to Vietnamese correctly", () => {
    expect(getStatusLabel("moi")).toBe("Mới");
    expect(getStatusLabel("loi_thoi")).toBe("Lỗi thời");
    expect(getStatusLabel("chua_render")).toBe("Chưa render");
    expect(getStatusLabel("loi")).toBe("Lỗi");
  });

  it("returns CSS variables for status colors without hardcoding", () => {
    const moi = getStatusStyles("moi");
    expect(moi.color).toContain("var(--cs-success)");
    expect(moi.bg).toContain("var(--cs-status-success-bg)");

    const stale = getStatusStyles("loi_thoi");
    expect(stale.color).toContain("var(--cs-warning)");
    expect(stale.bg).toContain("var(--cs-status-warning-bg)");

    const unrendered = getStatusStyles("chua_render");
    expect(unrendered.color).toContain("var(--cs-fg-muted)");

    const error = getStatusStyles("loi");
    expect(error.color).toContain("var(--cs-danger)");
  });

  it("configures stroke style based on edge role", () => {
    const uses = getEdgeRoleStyle("uses");
    expect(uses.strokeDasharray).toBeUndefined();

    const ref = getEdgeRoleStyle("reference");
    expect(ref.strokeDasharray).toBe("5 5");

    const variant = getEdgeRoleStyle("variant_of");
    expect(variant.strokeDasharray).toBe("2 3");
  });

  it("computes deterministic layout by topo depth", () => {
    const nodes: GraphNode[] = [
      { id: "script", kind: "text", title: "Script", status: "moi", pinned: false, notes: [] },
      { id: "timeline", kind: "timeline", title: "Timeline", status: "moi", pinned: false, notes: [] },
      { id: "thumb", kind: "design", title: "Thumbnail", status: "moi", pinned: false, notes: [] },
    ];

    // timeline uses script
    // thumb uses timeline
    const edges: GraphEdge[] = [
      { from: "timeline", to: "artifact:script", role: "uses" },
      { from: "thumb", to: "artifact:timeline", role: "uses" },
    ];

    const layout = computeGraphLayout(nodes, edges);
    expect(layout.nodes.length).toBe(3);

    const scriptNode = layout.nodes.find((n) => n.id === "script")!;
    const timelineNode = layout.nodes.find((n) => n.id === "timeline")!;
    const thumbNode = layout.nodes.find((n) => n.id === "thumb")!;

    expect(scriptNode.depth).toBe(0);
    expect(timelineNode.depth).toBe(1);
    expect(thumbNode.depth).toBe(2);

    expect(scriptNode.x).toBeLessThan(timelineNode.x);
    expect(timelineNode.x).toBeLessThan(thumbNode.x);
  });

  it("handles empty nodes gracefully", () => {
    const layout = computeGraphLayout([], []);
    expect(layout.nodes).toEqual([]);
    expect(layout.edges).toEqual([]);
  });
});
