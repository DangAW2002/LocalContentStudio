import React from "react";
import { Icon } from "../../../components/Icon";

export interface FileChangeItem {
  path: string;
  op: "create" | "modify" | "delete";
}

export interface AggregatedFileChange {
  path: string;
  op: "create" | "modify" | "delete";
  fileName: string;
  dirPath: string;
  category: "image" | "video" | "audio" | "text" | "other";
}

/**
 * Pure aggregation: merges file changes by path (last op wins) and sorts images first.
 */
export function aggregateFileChanges(changes: FileChangeItem[]): AggregatedFileChange[] {
  const map = new Map<string, "create" | "modify" | "delete">();
  for (const c of changes) {
    if (!c.path) continue;
    map.set(c.path, c.op);
  }

  const result: AggregatedFileChange[] = [];
  for (const [path, op] of map.entries()) {
    const normalized = path.replace(/\\/g, "/");
    const parts = normalized.split("/").filter(Boolean);
    const fileName = parts.pop() || normalized;
    const dirPath = parts.join("/");

    const ext = fileName.split(".").pop()?.toLowerCase() || "";
    let category: AggregatedFileChange["category"] = "other";
    if (["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "avif", "ico"].includes(ext)) {
      category = "image";
    } else if (["mp4", "webm", "mov", "mkv", "avi"].includes(ext)) {
      category = "video";
    } else if (["mp3", "wav", "ogg", "m4a", "flac", "aac"].includes(ext)) {
      category = "audio";
    } else if (
      [
        "md", "txt", "json", "yaml", "yml", "ts", "tsx", "js", "jsx",
        "html", "css", "rs", "toml", "sh", "py", "sql", "csv"
      ].includes(ext)
    ) {
      category = "text";
    }

    result.push({ path, op, fileName, dirPath, category });
  }

  // Sort: images first
  result.sort((a, b) => {
    const aIsImg = a.category === "image" ? 0 : 1;
    const bIsImg = b.category === "image" ? 0 : 1;
    return aIsImg - bIsImg;
  });

  return result;
}

export interface ResultCardProps {
  fileChanges: FileChangeItem[];
  onSelectFile: (path: string) => void;
}

const COLLAPSED_LIMIT = 6;

export const ResultCard: React.FC<ResultCardProps> = ({ fileChanges, onSelectFile }) => {
  const [showAll, setShowAll] = React.useState(false);

  const items = React.useMemo(() => aggregateFileChanges(fileChanges), [fileChanges]);
  if (items.length === 0) return null;

  const visibleItems = showAll || items.length <= COLLAPSED_LIMIT ? items : items.slice(0, COLLAPSED_LIMIT);
  const canCollapse = items.length > COLLAPSED_LIMIT;

  return (
    <div
      style={{
        marginTop: 8,
        border: "1px solid var(--cs-border-subtle)",
        borderRadius: "var(--cs-radius-control)",
        backgroundColor: "var(--cs-bg-panel)",
        padding: "8px 10px",
        display: "flex",
        flexDirection: "column",
        gap: 6,
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          fontSize: "var(--cs-text-xs)",
          color: "var(--cs-fg-muted)",
          fontWeight: 600,
        }}
      >
        <span>{`Kết quả · ${items.length} tệp`}</span>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {visibleItems.map((item) => {
          const isDelete = item.op === "delete";

          let iconName = "file";
          if (item.category === "image") iconName = "image";
          else if (item.category === "video") iconName = "video";
          else if (item.category === "audio") iconName = "audio";
          else if (item.category === "text") iconName = "file-text";

          const opLabel = item.op === "create" ? "Mới" : item.op === "delete" ? "Xoá" : "Sửa";
          const opColor =
            item.op === "create"
              ? "var(--cs-success)"
              : item.op === "delete"
              ? "var(--cs-danger)"
              : "var(--cs-accent)";
          const opBg =
            item.op === "create"
              ? "var(--cs-status-success-bg, rgba(46,160,67,0.15))"
              : item.op === "delete"
              ? "var(--cs-status-danger-bg, rgba(248,81,73,0.15))"
              : "var(--cs-status-info-bg, rgba(56,139,253,0.15))";

          return (
            <div
              key={item.path}
              onClick={isDelete ? undefined : () => onSelectFile(item.path)}
              role={isDelete ? undefined : "button"}
              tabIndex={isDelete ? undefined : 0}
              onKeyDown={
                isDelete
                  ? undefined
                  : (e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        onSelectFile(item.path);
                      }
                    }
              }
              title={isDelete ? `Đã xoá ${item.path}` : `Bấm để xem ${item.path}`}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 8,
                padding: "4px 8px",
                borderRadius: "var(--cs-radius-control)",
                backgroundColor: "var(--cs-bg-raised)",
                fontSize: "var(--cs-text-xs)",
                cursor: isDelete ? "default" : "pointer",
                textDecoration: isDelete ? "line-through" : "none",
                opacity: isDelete ? 0.6 : 1,
                userSelect: "none",
              }}
            >
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  minWidth: 0,
                  overflow: "hidden",
                }}
              >
                <Icon
                  name={iconName}
                  size={13}
                  color={item.category === "image" ? "var(--cs-accent)" : "var(--cs-fg-secondary)"}
                />
                <span
                  style={{
                    fontFamily: "var(--cs-font-mono)",
                    fontWeight: 500,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {item.fileName}
                </span>
                {item.dirPath && (
                  <span
                    style={{
                      color: "var(--cs-fg-muted)",
                      fontSize: 11,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {item.dirPath}/
                  </span>
                )}
              </div>

              <span
                style={{
                  fontSize: 10,
                  fontWeight: 600,
                  padding: "1px 5px",
                  borderRadius: "var(--cs-radius-control)",
                  color: opColor,
                  backgroundColor: opBg,
                  flexShrink: 0,
                }}
              >
                {opLabel}
              </span>
            </div>
          );
        })}
      </div>

      {canCollapse && (
        <button
          type="button"
          onClick={() => setShowAll(!showAll)}
          style={{
            background: "none",
            border: "none",
            padding: "4px 2px",
            fontSize: "var(--cs-text-xs)",
            color: "var(--cs-accent)",
            cursor: "pointer",
            textAlign: "left",
            alignSelf: "flex-start",
          }}
        >
          {showAll ? "Thu gọn" : `Xem tất cả (${items.length} tệp)`}
        </button>
      )}
    </div>
  );
};
