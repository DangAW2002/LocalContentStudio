import React from "react";
import { Dialog, Input } from "@studio/design-system";

export interface CommandItem {
  id: string;
  title: string;
  category: string;
  action: () => void;
}

export interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  commands: CommandItem[];
}

export const CommandPalette: React.FC<CommandPaletteProps> = ({
  open,
  onClose,
  commands,
}) => {
  const [query, setQuery] = React.useState("");

  const filtered = commands.filter((c) =>
    c.title.toLowerCase().includes(query.toLowerCase()) ||
    c.category.toLowerCase().includes(query.toLowerCase())
  );

  return (
    <Dialog open={open} title="Thao tác nhanh (Command Palette)" onClose={onClose}>
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
        <Input
          placeholder="Nhập lệnh hoặc màn hình cần mở..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          autoFocus
        />

        <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)", maxHeight: "280px", overflowY: "auto" }}>
          {filtered.length === 0 ? (
            <span style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)", padding: "var(--cs-space-2)" }}>
              Không tìm thấy lệnh nào phù hợp.
            </span>
          ) : (
            filtered.map((item) => (
              <button
                key={item.id}
                onClick={() => {
                  item.action();
                  onClose();
                }}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "var(--cs-space-2) var(--cs-space-3)",
                  borderRadius: "var(--cs-radius-sm)",
                  border: "none",
                  backgroundColor: "var(--cs-bg-raised)",
                  color: "var(--cs-fg-primary)",
                  cursor: "pointer",
                  textAlign: "left",
                  fontSize: "var(--cs-text-sm)",
                }}
              >
                <span>{item.title}</span>
                <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", textTransform: "uppercase" }}>
                  {item.category}
                </span>
              </button>
            ))
          )}
        </div>
      </div>
    </Dialog>
  );
};
