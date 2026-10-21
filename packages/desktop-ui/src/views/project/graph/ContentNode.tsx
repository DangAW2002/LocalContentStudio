import React from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { Icon } from "../../../components/Icon";
import { Badge } from "@studio/design-system";
import { getStatusStyles, getStatusLabel } from "./graph-layout";
import type { GraphNodeStatus } from "../../../api/types";

export interface CustomNodeData extends Record<string, unknown> {
  id: string;
  kind: string;
  title: string;
  status: GraphNodeStatus;
  pinned: boolean;
  notes: string[];
}

function getKindIcon(kind: string): string {
  switch (kind) {
    case "design":
      return "design";
    case "motion":
      return "motion";
    case "timeline":
      return "video";
    case "text":
      return "file-text";
    case "audio":
      return "music";
    case "generation":
      return "sparkles";
    case "asset":
      return "image";
    default:
      return "file";
  }
}

export const ContentNode: React.FC<NodeProps<any>> = React.memo(({ data, selected }) => {
  const nodeData = data as unknown as CustomNodeData;
  const statusStyles = getStatusStyles(nodeData.status);
  const statusLabel = getStatusLabel(nodeData.status);
  const iconName = getKindIcon(nodeData.kind);

  return (
    <div
      style={{
        width: 200,
        padding: "8px 12px",
        background: "var(--cs-bg-panel)",
        border: `1.5px solid ${selected ? "var(--cs-accent)" : "var(--cs-border-subtle)"}`,
        boxShadow: selected ? "0 0 0 2px var(--cs-accent-subtle)" : "0 1px 3px rgba(0,0,0,0.06)",
        borderRadius: "var(--cs-radius-control)",
        cursor: "pointer",
        position: "relative",
        userSelect: "none",
        fontFamily: "var(--cs-font-ui)",
      }}
    >
      <Handle
        type="target"
        position={Position.Left}
        style={{
          background: "var(--cs-fg-muted)",
          width: 6,
          height: 6,
          border: "none",
        }}
      />

      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 4 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
          <Icon name={iconName} size={14} color="var(--cs-fg-secondary)" />
          <span
            style={{
              fontSize: "var(--cs-text-xs)",
              color: "var(--cs-fg-muted)",
              textTransform: "uppercase",
              letterSpacing: "0.5px",
              fontWeight: 600,
            }}
          >
            {nodeData.kind}
          </span>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
          {nodeData.pinned && (
            <span title="Đã khóa: không tự động render lại">
              <Icon name="lock" size={12} color="var(--cs-warning)" />
            </span>
          )}
          <Badge
            variant={
              nodeData.status === "moi"
                ? "success"
                : nodeData.status === "loi_thoi"
                ? "warning"
                : nodeData.status === "loi"
                ? "danger"
                : "neutral"
            }
          >
            {statusLabel}
          </Badge>
        </div>
      </div>

      <div
        style={{
          fontSize: "var(--cs-text-sm)",
          fontWeight: 600,
          color: "var(--cs-fg-primary)",
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
        title={nodeData.title || nodeData.id}
      >
        {nodeData.title || nodeData.id}
      </div>

      <Handle
        type="source"
        position={Position.Right}
        style={{
          background: "var(--cs-fg-muted)",
          width: 6,
          height: 6,
          border: "none",
        }}
      />
    </div>
  );
});

ContentNode.displayName = "ContentNode";
