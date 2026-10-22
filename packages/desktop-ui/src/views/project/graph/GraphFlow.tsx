import React from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MarkerType,
  applyNodeChanges,
  applyEdgeChanges,
  type Node,
  type Edge,
  type Connection,
  type NodeChange,
  type EdgeChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { ContentNode } from "./ContentNode";
import { computeGraphLayout, getEdgeRoleStyle } from "./graph-layout";
import { Button, EmptyState } from "@studio/design-system";
import { Icon } from "../../../components/Icon";
import { studioApi } from "../../../api/client";
import type { ContentGraphResponse, DownstreamPlan } from "../../../api/types";
import { RolePickerDialog } from "./RolePickerDialog";
import { NotesDialog } from "./NotesDialog";
import { VariantDialog } from "./VariantDialog";
import { AttachProfileDialog } from "./AttachProfileDialog";
import { DownstreamPlanDialog } from "./DownstreamPlanDialog";

export interface GraphFlowProps {
  projectSlug: string;
  onSendToAgent?: (refs: string[]) => void;
  onOpenArtifact?: (artifactId: string) => void;
}

const nodeTypes: any = {
  content: ContentNode,
};

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
  const [errorToast, setErrorToast] = React.useState<string | null>(null);

  // Selection
  const [selectedNodeIds, setSelectedNodeIds] = React.useState<string[]>([]);
  const [selectedEdgeIds, setSelectedEdgeIds] = React.useState<string[]>([]);

  // Dialog states
  const [pendingConnect, setPendingConnect] = React.useState<{ source: string; target: string } | null>(null);
  const [notesTarget, setNotesTarget] = React.useState<{ id: string; notes: string[] } | null>(null);
  const [variantTarget, setVariantTarget] = React.useState<string | null>(null);
  const [attachTarget, setAttachTarget] = React.useState<string | null>(null);
  const [downstreamPlan, setDownstreamPlan] = React.useState<DownstreamPlan | null>(null);

  // Context menu state
  const [contextMenu, setContextMenu] = React.useState<{
    x: number;
    y: number;
    nodeId: string;
    pinned: boolean;
    notes: string[];
  } | null>(null);

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

  // Close context menu on outside click
  React.useEffect(() => {
    const handleOutside = () => setContextMenu(null);
    window.addEventListener("click", handleOutside);
    return () => window.removeEventListener("click", handleOutside);
  }, []);

  const { rfNodes, rfEdges } = React.useMemo(() => {
    if (!data || data.nodes.length === 0) {
      return { rfNodes: [], rfEdges: [] };
    }

    const layout = computeGraphLayout(data.nodes, data.edges);

    const nodes: Node[] = layout.nodes.map((n) => ({
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

    const edges: Edge[] = layout.edges.map((e) => {
      const style = getEdgeRoleStyle(e.role);
      const isSelected = selectedEdgeIds.includes(e.id);
      return {
        id: e.id,
        source: e.source,
        target: e.target,
        style: {
          stroke: isSelected ? "var(--cs-accent)" : style.stroke,
          strokeWidth: isSelected ? 2.5 : 1.5,
          strokeDasharray: style.strokeDasharray,
        },
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 14,
          height: 14,
          color: isSelected ? "var(--cs-accent)" : style.stroke,
        },
        selected: isSelected,
      };
    });

    return { rfNodes: nodes, rfEdges: edges };
  }, [data, selectedNodeIds, selectedEdgeIds]);

  const [nodes, setNodes] = React.useState<Node[]>([]);
  const [edges, setEdges] = React.useState<Edge[]>([]);

  React.useEffect(() => {
    setNodes(rfNodes);
    setEdges(rfEdges);
  }, [rfNodes, rfEdges]);

  const onNodesChange = React.useCallback(
    (changes: NodeChange[]) => setNodes((nds) => applyNodeChanges(changes, nds)),
    []
  );

  const onEdgesChange = React.useCallback(
    (changes: EdgeChange[]) => setEdges((eds) => applyEdgeChanges(changes, eds)),
    []
  );

  const handleSelectionChange = React.useCallback(
    ({ nodes: selNodes, edges: selEdges }: { nodes: Node[]; edges: Edge[] }) => {
      setSelectedNodeIds(selNodes.map((n) => n.id));
      setSelectedEdgeIds(selEdges.map((e) => e.id));
    },
    []
  );

  const handleNodeDoubleClick = React.useCallback(
    (_: React.MouseEvent, node: Node) => {
      if (onOpenArtifact) {
        onOpenArtifact(node.id);
      }
    },
    [onOpenArtifact]
  );

  const handleNodeContextMenu = React.useCallback(
    (event: React.MouseEvent, node: Node) => {
      event.preventDefault();
      const nodeData = node.data as any;
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        nodeId: node.id,
        pinned: Boolean(nodeData?.pinned),
        notes: Array.isArray(nodeData?.notes) ? nodeData.notes : [],
      });
    },
    []
  );

  // Connect handle
  const onConnect = React.useCallback(
    (connection: Connection) => {
      if (!connection.source || !connection.target) return;
      if (connection.source === connection.target) {
        setErrorToast("Không thể liên kết một node với chính nó.");
        return;
      }
      setPendingConnect({ source: connection.source, target: connection.target });
    },
    []
  );

  const handleRoleSelected = async (role: "uses" | "reference" | "variant_of") => {
    if (!pendingConnect) return;
    try {
      await studioApi.linkGraphEdge(projectSlug, pendingConnect.source, pendingConnect.target, role);
      await loadGraph();
    } catch (err: any) {
      setErrorToast(err?.message || "Lỗi khi tạo liên kết.");
    }
  };

  // Delete edge
  const handleDeleteSelected = React.useCallback(async () => {
    if (selectedEdgeIds.length === 0 || !data) return;
    for (const edgeId of selectedEdgeIds) {
      const e = data.edges.find((ed) => `${ed.from}->${ed.to}` === edgeId || ed.from === edgeId);
      if (e) {
        try {
          await studioApi.unlinkGraphEdge(projectSlug, e.from, e.to);
        } catch (err: any) {
          setErrorToast(err?.message || "Lỗi khi xóa liên kết.");
        }
      }
    }
    await loadGraph();
  }, [selectedEdgeIds, data, projectSlug, loadGraph]);

  // Context menu actions
  const handleTogglePin = async () => {
    if (!contextMenu) return;
    try {
      await studioApi.setGraphNodePinned(projectSlug, contextMenu.nodeId, !contextMenu.pinned);
      await loadGraph();
    } catch (err: any) {
      setErrorToast(err?.message || "Lỗi khi cập nhật trạng thái khóa.");
    }
    setContextMenu(null);
  };

  const handleOpenNotes = () => {
    if (!contextMenu) return;
    setNotesTarget({ id: contextMenu.nodeId, notes: contextMenu.notes });
    setContextMenu(null);
  };

  const handleSaveNotes = async (notes: string[]) => {
    if (!notesTarget) return;
    try {
      await studioApi.setGraphNodeNotes(projectSlug, notesTarget.id, notes);
      await loadGraph();
    } catch (err: any) {
      setErrorToast(err?.message || "Lỗi khi lưu ghi chú.");
    }
  };

  const handleOpenVariant = () => {
    if (!contextMenu) return;
    setVariantTarget(contextMenu.nodeId);
    setContextMenu(null);
  };

  const handleCreateVariant = async (newId: string, title?: string) => {
    if (!variantTarget) return;
    await studioApi.createGraphVariant(projectSlug, variantTarget, newId, title);
    await loadGraph();
  };

  const handleMakeReference = async () => {
    if (!contextMenu) return;
    const nodeId = contextMenu.nodeId;
    setContextMenu(null);
    try {
      await studioApi.makeGraphReference(projectSlug, nodeId);
      await loadGraph();
    } catch (err: any) {
      setErrorToast(err?.message || "Lỗi khi tạo hồ sơ.");
    }
  };

  const handleOpenAttach = () => {
    if (!contextMenu) return;
    setAttachTarget(contextMenu.nodeId);
    setContextMenu(null);
  };

  const handleAttachProfile = async (profileId: string) => {
    if (!attachTarget) return;
    try {
      await studioApi.linkGraphEdge(projectSlug, attachTarget, `artifact:${profileId}`, "reference");
      setAttachTarget(null);
      await loadGraph();
    } catch (err: any) {
      setErrorToast(err?.message || "Lỗi khi gắn hồ sơ.");
    }
  };

  const handleOpenDownstream = async () => {
    if (!contextMenu) return;
    const targetId = contextMenu.nodeId;
    setContextMenu(null);
    try {
      const plan = await studioApi.getGraphDownstreamPlan(projectSlug, targetId);
      setDownstreamPlan(plan);
    } catch (err: any) {
      setErrorToast(err?.message || "Lỗi khi phân tích cập nhật phía dưới.");
    }
  };

  const handleRunDownstream = async (runPaid: boolean) => {
    if (!downstreamPlan) return;
    try {
      await studioApi.runGraphDownstreamUpdate(projectSlug, downstreamPlan.targetRef, runPaid);
      await loadGraph();
    } catch (err: any) {
      setErrorToast(err?.message || "Lỗi khi chạy cập nhật phía dưới.");
    }
  };

  const handleRerender = async () => {
    if (!contextMenu) return;
    const artId = contextMenu.nodeId;
    setContextMenu(null);
    try {
      await studioApi.renderArtifact({ project: projectSlug, id: artId });
      await loadGraph();
    } catch (err: any) {
      setErrorToast(err?.message || "Lỗi khi render lại.");
    }
  };

  const handleOpenStage = () => {
    if (!contextMenu) return;
    if (onOpenArtifact) {
      onOpenArtifact(contextMenu.nodeId);
    }
    setContextMenu(null);
  };

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
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        background: "var(--cs-bg-canvas)",
        position: "relative",
      }}
    >
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
          {selectedEdgeIds.length > 0 && (
            <Button
              size="sm"
              variant="destructive"
              onClick={handleDeleteSelected}
              leftIcon={<Icon name="trash" size={13} />}
            >
              Xóa cạnh ({selectedEdgeIds.length})
            </Button>
          )}

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

      {/* Error Toast */}
      {errorToast && (
        <div
          style={{
            position: "absolute",
            top: 52,
            right: 16,
            zIndex: 1000,
            background: "var(--cs-danger)",
            color: "var(--cs-fg-on-accent)",
            padding: "8px 14px",
            borderRadius: "var(--cs-radius-control)",
            fontSize: "var(--cs-text-xs)",
            boxShadow: "0 2px 8px rgba(0,0,0,0.15)",
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <span>{errorToast}</span>
          <button
            type="button"
            onClick={() => setErrorToast(null)}
            style={{
              background: "transparent",
              border: "none",
              color: "inherit",
              cursor: "pointer",
              fontWeight: "bold",
            }}
          >
            ✕
          </button>
        </div>
      )}

      {/* ReactFlow Canvas */}
      <div style={{ flex: 1, minHeight: 0 }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          onSelectionChange={handleSelectionChange}
          onNodeDoubleClick={handleNodeDoubleClick}
          onNodeContextMenu={handleNodeContextMenu}
          onKeyDown={(e) => {
            if (e.key === "Delete" || e.key === "Backspace") {
              handleDeleteSelected();
            }
          }}
          nodesDraggable={true}
          nodesConnectable={true}
          elementsSelectable={true}
          fitView
          // Never shrink below a readable size: wider graphs are panned instead.
          fitViewOptions={{ padding: 0.15, maxZoom: 1.0, minZoom: 0.2 }}
          minZoom={0.2}
          maxZoom={1.5}
          colorMode={colorMode}
          style={{ background: "var(--cs-bg-canvas)" }}
        >
          <Background color="var(--cs-canvas-grid)" gap={20} size={1} />
          <Controls showInteractive={false} />
        </ReactFlow>
      </div>

      {/* Context Menu */}
      {contextMenu && (
        <div
          className="cs-menu"
          style={{
            position: "fixed",
            top: contextMenu.y,
            left: contextMenu.x,
            zIndex: 9999,
          }}
          onClick={(e) => e.stopPropagation()}
        >
          <button type="button" className="cs-menu-item" onClick={handleTogglePin}>
            {contextMenu.pinned ? "Mở khóa node" : "Khóa node (pinned)"}
          </button>
          <button type="button" className="cs-menu-item" onClick={handleOpenNotes}>
            Ghi chú...
          </button>
          <button type="button" className="cs-menu-item" onClick={handleOpenVariant}>
            Tạo biến thể
          </button>
          <button type="button" className="cs-menu-item" onClick={handleMakeReference}>
            Đặt làm hồ sơ
          </button>
          <button type="button" className="cs-menu-item" onClick={handleOpenAttach}>
            Gắn hồ sơ...
          </button>
          <button type="button" className="cs-menu-item" onClick={handleOpenDownstream}>
            Cập nhật phía dưới
          </button>
          <button type="button" className="cs-menu-item" onClick={handleRerender}>
            Render lại
          </button>
          <div style={{ height: 1, background: "var(--cs-border-subtle)", margin: "4px 0" }} />
          <button type="button" className="cs-menu-item" onClick={handleOpenStage}>
            Mở trên Stage
          </button>
        </div>
      )}

      {/* Dialogs */}
      {pendingConnect && (
        <RolePickerDialog
          open={Boolean(pendingConnect)}
          sourceId={pendingConnect.source}
          targetId={pendingConnect.target}
          onClose={() => setPendingConnect(null)}
          onSelectRole={handleRoleSelected}
        />
      )}

      {notesTarget && (
        <NotesDialog
          open={Boolean(notesTarget)}
          artifactId={notesTarget.id}
          initialNotes={notesTarget.notes}
          onClose={() => setNotesTarget(null)}
          onSave={handleSaveNotes}
        />
      )}

      {variantTarget && (
        <VariantDialog
          open={Boolean(variantTarget)}
          sourceId={variantTarget}
          onClose={() => setVariantTarget(null)}
          onCreate={handleCreateVariant}
        />
      )}

      {attachTarget && (
        <AttachProfileDialog
          open={Boolean(attachTarget)}
          targetNodeId={attachTarget}
          availableProfiles={(data?.nodes ?? [])
            .filter((n) => n.kind === "reference" && n.id !== attachTarget)
            .map((n) => ({ id: n.id, title: n.title }))}
          onClose={() => setAttachTarget(null)}
          onAttach={handleAttachProfile}
        />
      )}

      {downstreamPlan && (
        <DownstreamPlanDialog
          open={Boolean(downstreamPlan)}
          plan={downstreamPlan}
          onClose={() => setDownstreamPlan(null)}
          onRun={handleRunDownstream}
        />
      )}
    </div>
  );
};
