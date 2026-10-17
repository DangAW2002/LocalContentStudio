import React from "react";
import { Icon } from "../../components/Icon";
import { Badge, EmptyState } from "@studio/design-system";
import type { ProjectFileEntry } from "../../api/types";

export interface ProjectFolderTreeProps {
  files: ProjectFileEntry[];
  onSelectFile: (path: string) => void;
}

interface TreeNode {
  id: string;
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  kind: string;
  children: TreeNode[];
}

const FILE_KIND_LABELS_VI: Record<string, string> = {
  markdown: "Văn bản",
  text: "Văn bản",
  html: "HTML",
  code: "Mã nguồn",
  json: "JSON",
  image: "Hình ảnh",
  video: "Video",
  audio: "Âm thanh",
  unknown: "Khác",
};

function formatSize(bytes: number): string {
  if (bytes <= 0) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function buildTree(files: ProjectFileEntry[]): TreeNode[] {
  const map = new Map<string, TreeNode>();

  for (const f of files) {
    map.set(f.path, {
      id: f.path,
      name: f.name || f.path.split("/").pop() || f.path,
      path: f.path,
      isDir: f.isDir,
      size: f.size,
      kind: f.kind,
      children: [],
    });
  }

  // Ensure intermediate directory nodes exist
  for (const f of files) {
    const parts = f.path.split("/");
    for (let i = 1; i < parts.length; i++) {
      const parentPath = parts.slice(0, i).join("/");
      if (!map.has(parentPath)) {
        map.set(parentPath, {
          id: parentPath,
          name: parts[i - 1],
          path: parentPath,
          isDir: true,
          size: 0,
          kind: "folder",
          children: [],
        });
      }
    }
  }

  const roots: TreeNode[] = [];

  for (const [path, node] of map.entries()) {
    const lastSlash = path.lastIndexOf("/");
    if (lastSlash === -1) {
      roots.push(node);
    } else {
      const parentPath = path.slice(0, lastSlash);
      const parent = map.get(parentPath);
      if (parent) {
        if (!parent.children.some((c) => c.path === node.path)) {
          parent.children.push(node);
        }
      } else {
        roots.push(node);
      }
    }
  }

  function sortNodes(nodes: TreeNode[]) {
    nodes.sort((a, b) => {
      if (a.isDir !== b.isDir) {
        return a.isDir ? -1 : 1; // Folders first
      }
      return a.name.localeCompare(b.name, "vi", { sensitivity: "base" });
    });
    for (const n of nodes) {
      if (n.children.length > 0) {
        sortNodes(n.children);
      }
    }
  }

  sortNodes(roots);
  return roots;
}

export const ProjectFolderTree: React.FC<ProjectFolderTreeProps> = ({
  files,
  onSelectFile,
}) => {
  const [collapsedPaths, setCollapsedPaths] = React.useState<Set<string>>(new Set());

  const tree = React.useMemo(() => buildTree(files), [files]);

  const toggleFolder = (path: string) => {
    setCollapsedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  if (files.length === 0) {
    return (
      <EmptyState
        icon={<Icon name="package" size={24} />}
        title="Chưa có tệp nào"
        description="Khi agent tạo tệp hoặc thành phẩm, cây thư mục sẽ xuất hiện tại đây."
      />
    );
  }

  const renderNode = (node: TreeNode, depth: number): React.ReactNode => {
    const isCollapsed = collapsedPaths.has(node.path);

    if (node.isDir) {
      return (
        <div key={node.path} style={{ display: "flex", flexDirection: "column" }}>
          <div
            role="button"
            tabIndex={0}
            onClick={() => toggleFolder(node.path)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                toggleFolder(node.path);
              }
            }}
            aria-expanded={!isCollapsed}
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "6px var(--cs-space-3)",
              paddingLeft: 12 + depth * 16,
              borderBottom: "1px solid var(--cs-border-subtle)",
              cursor: "pointer",
              fontSize: "var(--cs-text-xs)",
              userSelect: "none",
              backgroundColor: "var(--cs-bg-panel)",
              transition: "background-color var(--cs-motion-duration) ease",
            }}
            onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = "var(--cs-bg-hover)")}
            onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "var(--cs-bg-panel)")}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
              <Icon
                name={isCollapsed ? "chev-right" : "chev-down"}
                size={12}
                color="var(--cs-fg-muted)"
              />
              <Icon name="folder" size={14} color="var(--cs-accent)" />
              <span
                style={{
                  fontWeight: 600,
                  color: "var(--cs-fg-primary)",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {node.name}
              </span>
            </div>

            <span style={{ fontSize: 11, color: "var(--cs-fg-muted)" }}>
              {node.children.length} mục
            </span>
          </div>

          {!isCollapsed && node.children.map((child) => renderNode(child, depth + 1))}
        </div>
      );
    }

    // File Node
    const kindLabel = FILE_KIND_LABELS_VI[node.kind] || "Khác";

    return (
      <div
        key={node.path}
        role="button"
        tabIndex={0}
        onClick={() => onSelectFile(node.path)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onSelectFile(node.path);
          }
        }}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "6px var(--cs-space-3)",
          paddingLeft: 28 + depth * 16,
          borderBottom: "1px solid var(--cs-border-subtle)",
          cursor: "pointer",
          fontSize: "var(--cs-text-xs)",
          transition: "background-color var(--cs-motion-duration) ease",
        }}
        onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = "var(--cs-bg-hover)")}
        onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "transparent")}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 6, minWidth: 0 }}>
          <Icon
            name={
              node.kind === "image"
                ? "image"
                : node.kind === "video"
                ? "video"
                : node.kind === "audio"
                ? "audio"
                : node.kind === "markdown" || node.kind === "text"
                ? "file-text"
                : node.kind === "code" || node.kind === "json" || node.kind === "html"
                ? "code"
                : "file"
            }
            size={13}
            color="var(--cs-fg-secondary)"
          />
          <span
            style={{
              fontWeight: 500,
              fontFamily: "var(--cs-font-mono)",
              color: "var(--cs-fg-primary)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {node.name}
          </span>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: 6, flexShrink: 0 }}>
          <Badge variant="neutral">{kindLabel}</Badge>
          {node.size > 0 && (
            <span style={{ color: "var(--cs-fg-muted)", fontSize: 11 }}>
              {formatSize(node.size)}
            </span>
          )}
        </div>
      </div>
    );
  };

  return (
    <div role="tree" aria-label="Cây thư mục tệp dự án" style={{ display: "flex", flexDirection: "column" }}>
      {tree.map((node) => renderNode(node, 0))}
    </div>
  );
};
