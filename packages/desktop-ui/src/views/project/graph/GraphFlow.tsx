import React from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MarkerType,
  type Node,
  type Edge,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { ContentNode } from "./ContentNode";
import { computeGraphLayout, getEdgeRoleStyle } from "./graph-layout";
import { Button, EmptyState } from "@studio/design-system";
import { Icon } from "../../../components/Icon";
import { studioApi } from "../../../api/client";
import type { ContentGraphResponse, GraphNode } from "../../../api/types";

export interface GraphFlowProps {
  projectSlug: string;
  onSendToAgent?: (refs: string[]) => void;
  onOpenArtifact?: (artifactId: string) => void;
}

const nodeTypes: any = {
  content: ContentNode,
};


/** The app theme (`data-theme` on <html>), not the OS one: the canvas must match the rest of the UI. */
function useAppColorMode(): "light" | "dark" {
  const read = () =>
    document.documentElement.getAttribute("data-theme") === "dark" ? "dark" : "light";
  const [mode, setMode] = React.useState<"light" | "dark">(read);
  React.useEffect(() => {
    const obs = new MutationObserver(() => setMode(read()));
    obs.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => obs.disconnect();
  }, []);
  return mode;
}

export const GraphFlow: React.FC<GraphFlowProps> = ({
  projectSlug,
  onSendToAgent,
  onOpenArtifact,
}) => {
  const colorMode = useAppColorMode();
  const [data, setData] = React.useState<ContentGraphResponse | null>(null);
  const [loading, setLoading] = React.useState(true);
  const [selectedNodeIds, setSelectedNodeIds] = React.useState<string[]>([]);

  const loadGraph = React.useCallback(async () => {
    if (!projectSlug) return;
    setLoading(true);
    try {
      const res = await studioApi.getGraph(projectSlug);
      setData(res);
    } catch {
      setData(null);
    } finally {
      setLoading(false);
    }
  }, [projectSlug]);

  React.useEffect(() => {
    loadGraph();
  }, [loadGraph]);

  const { nodes, edges } = React.useMemo(() => {
    if (!data || data.nodes.length === 0) {
      return { nodes: [], edges: [] };
    }

    const layout = computeGraphLayout(data.nodes, data.edges);

    const rfNodes: Node[] = layout.nodes.map((n) => ({
      id: n.id,
      type: "content",
      position: { x: n.x, y: n.y },
      data: {
        id: n.id,
        kind: n.kind,
        title: n.title,
        status: n.status,
        pinned: n.pinned,
        notes: n.notes,
      },
      selected: selectedNodeIds.includes(n.id),
    }));

    const rfEdges: Edge[] = layout.edges.map((e) => {
      const style = getEdgeRoleStyle(e.role);
      return {
        id: e.id,
        source: e.source,
        target: e.target,
        style: {
          stroke: style.stroke,
          strokeWidth: 1.5,
          strokeDasharray: style.strokeDasharray,
        },
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 14,
          height: 14,
          color: style.stroke,
        },
      };
    });

    return { nodes: rfNodes, edges: rfEdges };
  }, [data, selectedNodeIds]);

  const handleSelectionChange = React.useCallback(({ nodes: selectedNodes }: { nodes: Node[] }) => {
    setSelectedNodeIds(selectedNodes.map((n) => n.id));
  }, []);

  const handleNodeDoubleClick = React.useCallback(
    (_: React.MouseEvent, node: Node) => {
      if (onOpenArtifact) {
        onOpenArtifact(node.id);
      }
    },
    [onOpenArtifact]
  );

  const handleSendToAgent = React.useCallback(() => {
    if (onSendToAgent && selectedNodeIds.length > 0) {
      onSendToAgent(selectedNodeIds);
    }
  }, [onSendToAgent, selectedNodeIds]);

  if (loading && !data) {
    return (
      <div style={{ padding: "var(--cs-space-4)", color: "var(--cs-fg-muted)" }}>
        Đang tải sơ đồ nội dung...
      </div>
    );
  }

  if (!data || data.nodes.length === 0) {
    return (
      <div style={{ height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }}>
        <EmptyState
          title="Chưa có artifact nào"
          description="Dự án này chưa tạo artifact. Hãy tạo kịch bản, thiết kế hoặc timeline bằng agent hoặc nút tạo mới."
        />
      </div>
    );
  }

  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", background: "var(--cs-bg-canvas)" }}>
      {/* Top Toolbar */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "8px 16px",
          background: "var(--cs-bg-panel)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          gap: 12,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
          <span style={{ fontSize: "var(--cs-text-sm)", fontWeight: 600, color: "var(--cs-fg-primary)" }}>
            Sơ đồ liên kết ({data.nodes.length} node)
          </span>

          {/* Role Legend */}
          <div style={{ display: "flex", alignItems: "center", gap: 12, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
            <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <span style={{ width: 16, height: 2, background: "var(--cs-fg-secondary)", display: "inline-block" }} />
              uses (trực tiếp)
            </span>
            <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <span style={{ width: 16, height: 2, borderTop: "2px dashed var(--cs-accent)", display: "inline-block" }} />
              reference (hồ sơ)
            </span>
            <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <span style={{ width: 16, height: 2, borderTop: "2px dotted var(--cs-fg-muted)", display: "inline-block" }} />
              variant_of (biến thể)
            </span>
          </div>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {selectedNodeIds.length > 0 && (
            <Button
              size="sm"
              variant="primary"
              onClick={handleSendToAgent}
              leftIcon={<Icon name="sparkles" size={13} />}
            >
              Gửi cho agent ({selectedNodeIds.length})
            </Button>
          )}

          <Button size="sm" variant="quiet" onClick={loadGraph} leftIcon={<Icon name="refresh" size={13} />}>
            Làm mới
          </Button>
        </div>
      </div>

      {/* ReactFlow Canvas */}
      <div style={{ flex: 1, minHeight: 0 }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          onSelectionChange={handleSelectionChange}
          onNodeDoubleClick={handleNodeDoubleClick}
          nodesDraggable={false}
          nodesConnectable={false}
          elementsSelectable={true}
          fitView
          fitViewOptions={{ padding: 0.2, maxZoom: 1.2 }}
          minZoom={0.2}
          maxZoom={1.5}
          colorMode={colorMode}
          style={{ background: "var(--cs-bg-canvas)" }}
        >
          <Background color="var(--cs-canvas-grid)" gap={20} size={1} />
          <Controls showInteractive={false} />
        </ReactFlow>
      </div>
    </div>
  );
};
