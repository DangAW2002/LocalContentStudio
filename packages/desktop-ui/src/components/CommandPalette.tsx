import React from "react";
import { Icon } from "./Icon";
import { Kbd } from "@studio/design-system";
import { fuzzyScore, type FuzzyScoreResult } from "../runtime/fuzzy";
import {
  globalCommandRegistry,
  type CommandDefinition,
  type CommandContext,
} from "../runtime/commands";
import type { ProjectFileEntry, ThreadSummary } from "../api/types";

export interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  context?: CommandContext;
  projectFiles?: ProjectFileEntry[];
  threads?: ThreadSummary[];
  onSelectFile?: (path: string) => void;
  onSelectThread?: (threadId: string) => void;
  onOpenShortcutsList?: () => void;
}

interface PaletteItem {
  id: string;
  type: "command" | "file" | "thread" | "shortcut";
  title: string;
  category: string;
  icon?: string;
  shortcut?: string;
  matches?: number[];
  action: () => void;
}

export const CommandPalette: React.FC<CommandPaletteProps> = ({
  open,
  onClose,
  context = {},
  projectFiles = [],
  threads = [],
  onSelectFile,
  onSelectThread,
  onOpenShortcutsList,
}) => {
  const [query, setQuery] = React.useState("");
  const [selectedIndex, setSelectedIndex] = React.useState(0);
  const inputRef = React.useRef<HTMLInputElement>(null);
  const listRef = React.useRef<HTMLDivElement>(null);

  React.useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  // Determine mode based on prefix:
  // '@' -> File search
  // '#' -> Thread search
  // '?' -> Shortcuts list
  // '>' or default -> Command search
  const trimmed = query.trimStart();
  let mode: "command" | "file" | "thread" | "shortcut" = "command";
  let searchWord = query;

  if (trimmed.startsWith("@")) {
    mode = "file";
    searchWord = trimmed.slice(1).trimStart();
  } else if (trimmed.startsWith("#")) {
    mode = "thread";
    searchWord = trimmed.slice(1).trimStart();
  } else if (trimmed.startsWith("?")) {
    mode = "shortcut";
    searchWord = trimmed.slice(1).trimStart();
  } else if (trimmed.startsWith(">")) {
    mode = "command";
    searchWord = trimmed.slice(1).trimStart();
  }

  const items = React.useMemo<PaletteItem[]>(() => {
    if (!open) return [];

    if (mode === "file") {
      const filesOnly = projectFiles.filter((f) => !f.isDir);
      if (!searchWord) {
        return filesOnly.slice(0, 30).map((f) => ({
          id: `file:${f.path}`,
          type: "file",
          title: f.path,
          category: "Tệp dự án",
          icon: "file",
          action: () => onSelectFile?.(f.path),
        }));
      }
      return filesOnly
        .map((f) => {
          const res = fuzzyScore(searchWord, f.path);
          return { f, res };
        })
        .filter((item): item is { f: ProjectFileEntry; res: FuzzyScoreResult } => item.res !== null)
        .sort((a, b) => b.res.score - a.res.score)
        .slice(0, 30)
        .map(({ f, res }) => ({
          id: `file:${f.path}`,
          type: "file",
          title: f.path,
          category: "Tệp dự án",
          icon: "file",
          matches: res.matches,
          action: () => onSelectFile?.(f.path),
        }));
    }

    if (mode === "thread") {
      if (!searchWord) {
        return threads.map((t, idx) => ({
          id: `thread:${t.id}`,
          type: "thread",
          title: t.title?.trim() || `Hội thoại ${threads.length - idx}`,
          category: `Engine: ${t.engine === "claude" ? "Claude" : "agy"}`,
          icon: "message-square",
          action: () => onSelectThread?.(t.id),
        }));
      }
      return threads
        .map((t, idx) => {
          const title = t.title?.trim() || `Hội thoại ${threads.length - idx}`;
          const res = fuzzyScore(searchWord, title);
          return { t, title, res };
        })
        .filter((item): item is { t: ThreadSummary; title: string; res: FuzzyScoreResult } => item.res !== null)
        .sort((a, b) => b.res.score - a.res.score)
        .map(({ t, title, res }) => ({
          id: `thread:${t.id}`,
          type: "thread",
          title,
          category: `Engine: ${t.engine === "claude" ? "Claude" : "agy"}`,
          icon: "message-square",
          matches: res.matches,
          action: () => onSelectThread?.(t.id),
        }));
    }

    if (mode === "shortcut") {
      const allWithShortcuts = globalCommandRegistry
        .getAll()
        .filter((c) => Boolean(c.keybinding));
      if (!searchWord) {
        return allWithShortcuts.map((c) => ({
          id: `shortcut:${c.id}`,
          type: "shortcut",
          title: c.title,
          category: c.category,
          shortcut: c.keybinding,
          icon: c.icon || "command",
          action: () => {
            globalCommandRegistry.recordUsage(c.id);
            c.run(context);
          },
        }));
      }
      return allWithShortcuts
        .map((c) => {
          const res = fuzzyScore(searchWord, c.title);
          return { c, res };
        })
        .filter((item): item is { c: CommandDefinition; res: FuzzyScoreResult } => item.res !== null)
        .sort((a, b) => b.res.score - a.res.score)
        .map(({ c, res }) => ({
          id: `shortcut:${c.id}`,
          type: "shortcut",
          title: c.title,
          category: c.category,
          shortcut: c.keybinding,
          icon: c.icon || "command",
          matches: res.matches,
          action: () => {
            globalCommandRegistry.recordUsage(c.id);
            c.run(context);
          },
        }));
    }

    // Default: Command Mode
    const visible = globalCommandRegistry.getVisible(context);

    // If query is empty, show MRU commands first, followed by all commands
    if (!searchWord) {
      const mru = globalCommandRegistry.getMruCommands(context);
      const mruIdSet = new Set(mru.map((m) => m.id));
      const rest = visible.filter((c) => !mruIdSet.has(c.id));

      const mruItems: PaletteItem[] = mru.map((c) => ({
        id: `cmd:${c.id}`,
        type: "command",
        title: c.title,
        category: `Gần đây · ${c.category}`,
        icon: c.icon || "command",
        shortcut: c.keybinding,
        action: () => {
          globalCommandRegistry.recordUsage(c.id);
          c.run(context);
        },
      }));

      const restItems: PaletteItem[] = rest.map((c) => ({
        id: `cmd:${c.id}`,
        type: "command",
        title: c.title,
        category: c.category,
        icon: c.icon || "command",
        shortcut: c.keybinding,
        action: () => {
          globalCommandRegistry.recordUsage(c.id);
          c.run(context);
        },
      }));

      return [...mruItems, ...restItems];
    }

    // Fuzzy search commands by title and category
    return visible
      .map((c) => {
        const titleScore = fuzzyScore(searchWord, c.title);
        const catScore = fuzzyScore(searchWord, c.category);
        const best =
          titleScore && catScore
            ? titleScore.score >= catScore.score
              ? titleScore
              : catScore
            : titleScore || catScore;
        return { c, res: best };
      })
      .filter((item): item is { c: CommandDefinition; res: FuzzyScoreResult } => item.res !== null)
      .sort((a, b) => b.res.score - a.res.score)
      .map(({ c, res }) => ({
        id: `cmd:${c.id}`,
        type: "command",
        title: c.title,
        category: c.category,
        icon: c.icon || "command",
        shortcut: c.keybinding,
        matches: res.matches,
        action: () => {
          globalCommandRegistry.recordUsage(c.id);
          c.run(context);
        },
      }));
  }, [open, mode, searchWord, context, projectFiles, threads, onSelectFile, onSelectThread]);

  React.useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  // Keep selected item scrolled into view
  React.useEffect(() => {
    if (!listRef.current) return;
    const selectedEl = listRef.current.querySelector<HTMLElement>('[aria-selected="true"]');
    if (selectedEl) {
      selectedEl.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  if (!open) return null;

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Tab") {
      // Prevent focus escape
      e.preventDefault();
    } else if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) => (items.length > 0 ? (prev + 1 < items.length ? prev + 1 : 0) : 0));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((prev) => (items.length > 0 ? (prev - 1 >= 0 ? prev - 1 : items.length - 1) : 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (items[selectedIndex]) {
        items[selectedIndex].action();
        onClose();
      }
    }
  };

  /** Render text with matched letters highlighted */
  const renderHighlightedTitle = (title: string, matches?: number[]) => {
    if (!matches || matches.length === 0) {
      return <span>{title}</span>;
    }
    const matchSet = new Set(matches);
    const chars = Array.from(title);
    return (
      <span>
        {chars.map((ch, idx) =>
          matchSet.has(idx) ? (
            <strong
              key={idx}
              style={{
                color: "var(--cs-accent)",
                fontWeight: 700,
                textDecoration: "underline",
              }}
            >
              {ch}
            </strong>
          ) : (
            <span key={idx}>{ch}</span>
          )
        )}
      </span>
    );
  };

  const getPlaceholder = () => {
    if (mode === "file") return "Tìm tệp trong dự án (@)...";
    if (mode === "thread") return "Tìm hội thoại (#)...";
    if (mode === "shortcut") return "Tìm phím tắt (?)...";
    return "Gõ lệnh (>), tệp (@), thread (#) hoặc phím tắt (?)...";
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
          width: 620,
          maxWidth: "92vw",
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
          <Icon
            name={
              mode === "file"
                ? "file"
                : mode === "thread"
                ? "message-square"
                : mode === "shortcut"
                ? "command"
                : "search"
            }
            size={18}
            color="var(--cs-accent)"
          />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={getPlaceholder()}
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
          <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <span
              style={{
                fontSize: "11px",
                color: "var(--cs-fg-muted)",
                userSelect: "none",
              }}
            >
              ↑↓ di chuyển · Enter chọn
            </span>
            <Kbd>Esc</Kbd>
          </div>
        </div>

        {/* Results List */}
        <div
          ref={listRef}
          role="listbox"
          style={{
            maxHeight: 380,
            overflowY: "auto",
            padding: "4px 0",
          }}
        >
          {items.length === 0 ? (
            <div
              style={{
                padding: "var(--cs-space-6)",
                textAlign: "center",
                color: "var(--cs-fg-muted)",
                fontSize: "var(--cs-text-sm)",
              }}
            >
              {searchWord
                ? `Không tìm thấy kết quả nào khớp với "${searchWord}".`
                : "Không có mục nào."}
            </div>
          ) : (
            items.map((item, idx) => {
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
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 8,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    <Icon name={item.icon || "command"} size={16} />
                    {renderHighlightedTitle(item.title, item.matches)}
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
                    <span
                      style={{
                        fontSize: "var(--cs-text-xs)",
                        color: "var(--cs-fg-muted)",
                      }}
                    >
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
