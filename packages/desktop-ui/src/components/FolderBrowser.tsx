import React from "react";
import { Icon } from "./Icon";
import { Button, Callout } from "@studio/design-system";
import type { DirListing } from "../api/types";
import { studioApi } from "../api/client";

export interface FolderBrowserProps {
  /** Directory to start in; defaults to the home folder of the machine running Studio. */
  initialPath?: string;
  /** Label of the confirm button. */
  selectLabel?: string;
  onSelect: (path: string) => void;
  onCancel: () => void;
}

/**
 * Folder browser for the web build: a browser cannot show a native picker for
 * folders on the server, so this lists directories through `fs_list_dirs`.
 */
export const FolderBrowser: React.FC<FolderBrowserProps> = ({
  initialPath,
  selectLabel = "Chọn thư mục này",
  onSelect,
  onCancel,
}) => {
  const [listing, setListing] = React.useState<DirListing | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(true);

  const load = React.useCallback(async (path?: string) => {
    setLoading(true);
    setError(null);
    try {
      setListing(await studioApi.listDirs(path));
    } catch (err: any) {
      setError(err?.message || "Không đọc được thư mục.");
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    void load(initialPath?.trim() || undefined);
  }, [load, initialPath]);

  const rowStyle: React.CSSProperties = {
    display: "flex",
    alignItems: "center",
    gap: 8,
    width: "100%",
    padding: "6px 10px",
    background: "none",
    border: "none",
    borderBottom: "1px solid var(--cs-border-subtle)",
    color: "var(--cs-fg-primary)",
    fontSize: "var(--cs-text-sm)",
    textAlign: "left",
    cursor: "pointer",
  };

  return (
    <div
      aria-label="Duyệt thư mục"
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--cs-space-2)",
        border: "1px solid var(--cs-border-subtle)",
        borderRadius: "var(--cs-radius-control)",
        padding: "var(--cs-space-2)",
        backgroundColor: "var(--cs-bg-raised)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <Button
          size="sm"
          variant="secondary"
          disabled={!listing?.parent || loading}
          onClick={() => listing?.parent && load(listing.parent)}
          aria-label="Lên thư mục cha"
        >
          ..
        </Button>
        <Button
          size="sm"
          variant="secondary"
          disabled={!listing || loading}
          onClick={() => listing && load(listing.home)}
        >
          Home
        </Button>
        <span
          title={listing?.path}
          style={{
            flex: 1,
            minWidth: 0,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            fontFamily: "var(--cs-font-mono)",
            fontSize: "var(--cs-text-xs)",
            color: "var(--cs-fg-secondary)",
          }}
        >
          {listing?.path ?? "…"}
        </span>
      </div>

      {error && (
        <Callout variant="danger" title="Không đọc được thư mục">
          {error}
        </Callout>
      )}

      <div style={{ maxHeight: 220, overflowY: "auto" }}>
        {loading && !listing ? (
          <div style={{ padding: 8, color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-sm)" }}>Đang tải…</div>
        ) : listing && listing.entries.length === 0 ? (
          <div style={{ padding: 8, color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-sm)" }}>
            Không có thư mục con.
          </div>
        ) : (
          listing?.entries.map((entry) => (
            <button key={entry.path} type="button" style={rowStyle} onClick={() => load(entry.path)}>
              <Icon name="folder" size={14} color={entry.isWorkspace ? "var(--cs-accent)" : "var(--cs-fg-secondary)"} />
              <span style={{ flex: 1 }}>{entry.name}</span>
              {entry.isWorkspace && (
                <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-accent)", fontWeight: 600 }}>
                  Workspace
                </span>
              )}
            </button>
          ))
        )}
      </div>

      <div style={{ display: "flex", justifyContent: "flex-end", gap: "var(--cs-space-2)" }}>
        <Button size="sm" variant="secondary" onClick={onCancel}>
          Hủy
        </Button>
        <Button size="sm" variant="primary" disabled={!listing || loading} onClick={() => listing && onSelect(listing.path)}>
          {selectLabel}
        </Button>
      </div>
    </div>
  );
};
