import React from "react";
import { Icon } from "./Icon";
import { Kbd } from "@studio/design-system";

export interface CommandItem {
  id: string;
  title: string;
  category: string;
  icon?: string;
  shortcut?: string;
  action: () => void;
}

export interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  commands: CommandItem[];
}

function removeAccents(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/đ/g, "d")
    .replace(/Đ/g, "D")
    .toLowerCase();
}

export const CommandPalette: React.FC<CommandPaletteProps> = ({
  open,
  onClose,
  commands,
}) => {
  const [query, setQuery] = React.useState("");
  const [selectedIndex, setSelectedIndex] = React.useState(0);
  const inputRef = React.useRef<HTMLInputElement>(null);

  React.useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  if (!open) return null;

  const normalizedQuery = removeAccents(query.trim());
  const filtered = commands.filter((c) => {
    if (!normalizedQuery) return true;
    const title = removeAccents(c.title);
    const cat = removeAccents(c.category);
    return title.includes(normalizedQuery) || cat.includes(normalizedQuery);
  });

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) => (prev + 1 < filtered.length ? prev + 1 : 0));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((prev) => (prev - 1 >= 0 ? prev - 1 : filtered.length - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (filtered[selectedIndex]) {
        filtered[selectedIndex].action();
        onClose();
      }
    }
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Command Palette"
      onClick={onClose}
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "var(--cs-overlay)",
        display: "flex",
        alignItems: "flex-start",
        justifyContent: "center",
        paddingTop: "12vh",
        zIndex: 50,
      }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 560,
          maxWidth: "90vw",
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-strong)",
          boxShadow: "var(--cs-shadow-overlay)",
          borderRadius: "var(--cs-radius-none)",
          overflow: "hidden",
          display: "flex",
          flexDirection: "column",
        }}
      >
        {/* Search Input Bar */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "var(--cs-space-2)",
            padding: "var(--cs-space-3)",
            borderBottom: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-raised)",
          }}
        >
          <Icon name="search" size={18} color="var(--cs-accent)" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            onKeyDown={handleKeyDown}
            placeholder="Gõ lệnh hoặc điều hướng..."
            style={{
              flex: 1,
              background: "none",
              border: "none",
              outline: "none",
              color: "var(--cs-fg-primary)",
              fontFamily: "var(--cs-font-ui)",
              fontSize: "var(--cs-text-md)",
            }}
          />
          <Kbd>Esc</Kbd>
        </div>

        {/* Results List */}
        <div
          role="listbox"
          style={{
            maxHeight: 340,
            overflowY: "auto",
            padding: "4px 0",
          }}
        >
          {filtered.length === 0 ? (
            <div
              style={{
                padding: "var(--cs-space-6)",
                textAlign: "center",
                color: "var(--cs-fg-muted)",
                fontSize: "var(--cs-text-sm)",
              }}
            >
              Không tìm thấy lệnh hoặc trang nào khớp với "{query}".
            </div>
          ) : (
            filtered.map((item, idx) => {
              const isSelected = idx === selectedIndex;
              return (
                <div
                  key={item.id}
                  role="option"
                  aria-selected={isSelected}
                  onClick={() => {
                    item.action();
                    onClose();
                  }}
                  onMouseEnter={() => setSelectedIndex(idx)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "8px var(--cs-space-4)",
                    cursor: "pointer",
                    backgroundColor: isSelected ? "var(--cs-selection)" : "transparent",
                    color: isSelected ? "var(--cs-accent)" : "var(--cs-fg-primary)",
                    borderLeft: isSelected ? "2px solid var(--cs-accent)" : "2px solid transparent",
                    fontSize: "var(--cs-text-sm)",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <Icon name={item.icon || "command"} size={16} />
                    <span>{item.title}</span>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                      {item.category}
                    </span>
                    {item.shortcut && <Kbd>{item.shortcut}</Kbd>}
                  </div>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
};
