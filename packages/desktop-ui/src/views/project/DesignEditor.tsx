import React from "react";
import { Button, Badge } from "@studio/design-system";
import { Icon } from "../../components/Icon";
import { detectArtboardSize } from "../../utils/artboard";

/** Imperative API used by the Stage toolbar. */
export interface DesignEditorHandle {
  /** Clean HTML of the current document (editor marks removed). */
  serialize(): string;
}

export interface DesignEditorProps {
  /** HTML source of the artboard. Changing it reloads the editor and clears history. */
  html: string;
  /** Called whenever the unsaved state changes. */
  onDirtyChange: (dirty: boolean) => void;
  /** Ctrl+S inside the editor. */
  onSaveRequest: () => void;
  /** "Hỏi AI về layer này". */
  onAskAi: (layerId: string, text: string) => void;
}

interface LayerInfo {
  id: string;
  tag: string;
  isText: boolean;
}

interface InspectorState {
  text: string;
  left: number;
  top: number;
  width: number;
  height: number;
  color: string;
  background: string;
  fontSize: number;
}

const EDITOR_STYLE_ID = "lcs-editor-style";
const SELECTED_ATTR = "data-lcs-selected";

function toHex(color: string): string {
  const m = color.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/i);
  if (!m) return color.startsWith("#") ? color.slice(0, 7) : "";
  return "#" + [m[1], m[2], m[3]].map((v) => Number(v).toString(16).padStart(2, "0")).join("");
}

function px(v: string): number {
  const n = parseFloat(v);
  return Number.isFinite(n) ? Math.round(n) : 0;
}

/** Visual editor for `kind: design` HTML artboards: layers are elements marked with `data-layer`. */
export const DesignEditor = React.forwardRef<DesignEditorHandle, DesignEditorProps>(
  ({ html, onDirtyChange, onSaveRequest, onAskAi }, ref) => {
    const iframeRef = React.useRef<HTMLIFrameElement>(null);
    const containerRef = React.useRef<HTMLDivElement>(null);
    const [srcDoc, setSrcDoc] = React.useState(html);
    const [reloadKey, setReloadKey] = React.useState(0);
    const [layers, setLayers] = React.useState<LayerInfo[]>([]);
    const [selectedId, setSelectedId] = React.useState<string | null>(null);
    const [inspector, setInspector] = React.useState<InspectorState | null>(null);
    const [size, setSize] = React.useState(() => detectArtboardSize(html));
    const [scale, setScale] = React.useState(1);
    const undoStack = React.useRef<string[]>([]);
    const redoStack = React.useRef<string[]>([]);
    const editing = React.useRef(false);
    const [, forceRender] = React.useReducer((x: number) => x + 1, 0);

    const doc = () => iframeRef.current?.contentDocument ?? null;
    const layerEl = (id: string | null) =>
      id ? (doc()?.querySelector(`[data-layer="${CSS.escape(id)}"]`) as HTMLElement | null) : null;

    const serialize = React.useCallback((): string => {
      const d = doc();
      if (!d) return srcDoc;
      const clone = d.documentElement.cloneNode(true) as HTMLElement;
      clone.querySelector(`#${EDITOR_STYLE_ID}`)?.remove();
      clone.querySelectorAll(`[${SELECTED_ATTR}]`).forEach((el) => el.removeAttribute(SELECTED_ATTR));
      return "<!DOCTYPE html>\n" + clone.outerHTML;
    }, [srcDoc]);

    React.useImperativeHandle(ref, () => ({ serialize }), [serialize]);

    // External source change (load, agent edit, reload): reset history.
    React.useEffect(() => {
      setSrcDoc(html);
      setSize(detectArtboardSize(html));
      undoStack.current = [];
      redoStack.current = [];
      onDirtyChange(false);
      setReloadKey((k) => k + 1);
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [html]);

    const readInspector = (el: HTMLElement): InspectorState => {
      const cs = el.ownerDocument.defaultView!.getComputedStyle(el);
      return {
        text: el.children.length === 0 ? el.textContent ?? "" : "",
        left: px(el.style.left || cs.left),
        top: px(el.style.top || cs.top),
        width: px(el.style.width || cs.width),
        height: px(el.style.height || cs.height),
        color: toHex(cs.color),
        background: toHex(cs.backgroundColor),
        fontSize: px(cs.fontSize),
      };
    };

    const select = React.useCallback((id: string | null) => {
      const d = doc();
      if (!d) return;
      d.querySelectorAll(`[${SELECTED_ATTR}]`).forEach((el) => el.removeAttribute(SELECTED_ATTR));
      const el = layerEl(id);
      if (el) {
        el.setAttribute(SELECTED_ATTR, "");
        setInspector(readInspector(el));
      } else {
        setInspector(null);
      }
      setSelectedId(el ? id : null);
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);

    /** Snapshot for undo once per editing gesture, then mutate the selected layer. */
    const applyEdit = (mutate: (el: HTMLElement) => void, snapshot = true) => {
      const el = layerEl(selectedId);
      if (!el) return;
      if (snapshot) {
        undoStack.current.push(serialize());
        redoStack.current = [];
      }
      mutate(el);
      setInspector(readInspector(el));
      onDirtyChange(true);
      forceRender();
    };

    const fieldEdit = (mutate: (el: HTMLElement) => void) => {
      applyEdit(mutate, !editing.current);
      editing.current = true;
    };

    const ensurePositioned = (el: HTMLElement) => {
      const cs = el.ownerDocument.defaultView!.getComputedStyle(el);
      if (cs.position === "static") {
        el.style.position = "absolute";
        el.style.left = `${el.offsetLeft}px`;
        el.style.top = `${el.offsetTop}px`;
      }
    };

    const move = (dx: number, dy: number) =>
      applyEdit((el) => {
        ensurePositioned(el);
        el.style.left = `${px(el.style.left) + dx}px`;
        el.style.top = `${px(el.style.top) + dy}px`;
      });

    const restore = (from: React.MutableRefObject<string[]>, to: React.MutableRefObject<string[]>) => {
      const prev = from.current.pop();
      if (prev === undefined) return;
      to.current.push(serialize());
      setSrcDoc(prev);
      setReloadKey((k) => k + 1);
      onDirtyChange(true);
    };

    const handleKey = (e: KeyboardEvent | React.KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) return;
      const ctrl = e.ctrlKey || e.metaKey;
      if (ctrl && e.key.toLowerCase() === "s") {
        e.preventDefault();
        onSaveRequest();
      } else if (ctrl && e.key.toLowerCase() === "z") {
        e.preventDefault();
        restore(undoStack, redoStack);
      } else if (ctrl && e.key.toLowerCase() === "y") {
        e.preventDefault();
        restore(redoStack, undoStack);
      } else if (e.key === "Tab" && layers.length > 0) {
        e.preventDefault();
        const i = layers.findIndex((l) => l.id === selectedId);
        const next = layers[(i + (e.shiftKey ? -1 + layers.length : 1)) % layers.length];
        select(next.id);
      } else if (e.key === "Escape") {
        select(null);
      } else if (selectedId && e.key.startsWith("Arrow")) {
        e.preventDefault();
        const step = e.altKey ? 10 : 1;
        const d = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] }[e.key];
        if (d) move(d[0], d[1]);
      }
    };

    const fit = React.useCallback(() => {
      const box = containerRef.current;
      if (!box) return;
      const s = Math.min((box.clientWidth - 32) / size.w, (box.clientHeight - 32) / size.h, 1);
      setScale(s > 0 ? s : 1);
    }, [size]);

    React.useEffect(() => {
      fit();
      window.addEventListener("resize", fit);
      return () => window.removeEventListener("resize", fit);
    }, [fit]);

    const handleLoad = () => {
      const d = doc();
      if (!d) return;
      const focus = getComputedStyle(document.documentElement).getPropertyValue("--cs-focus").trim() || "Highlight";
      const style = d.createElement("style");
      style.id = EDITOR_STYLE_ID;
      style.textContent = `[data-layer]{cursor:pointer}[${SELECTED_ATTR}]{outline:2px solid ${focus} !important;outline-offset:2px}`;
      d.head.appendChild(style);
      const detected = detectArtboardSize(srcDoc);
      const w = Math.max(detected.w, d.documentElement.scrollWidth, d.body?.scrollWidth ?? 0);
      const h = Math.max(detected.h, d.documentElement.scrollHeight, d.body?.scrollHeight ?? 0);
      if (w > 0 && h > 0) setSize({ w, h });
      setLayers(
        Array.from(d.querySelectorAll<HTMLElement>("[data-layer]")).map((el) => ({
          id: el.getAttribute("data-layer") || "",
          tag: el.tagName.toLowerCase(),
          isText: el.children.length === 0,
        }))
      );
      d.addEventListener(
        "click",
        (ev) => {
          const el = (ev.target as Element | null)?.closest?.("[data-layer]");
          ev.preventDefault();
          select(el ? el.getAttribute("data-layer") : null);
        },
        true
      );
      d.addEventListener("keydown", (ev) => handleKey(ev));
      if (selectedId) select(selectedId);
    };

    const numField = (label: string, key: "left" | "top" | "width" | "height" | "fontSize") => (
      <label style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
        {label}
        <input
          type="number"
          value={inspector ? inspector[key] : 0}
          onBlur={() => (editing.current = false)}
          onChange={(e) => {
            const v = Number(e.target.value);
            fieldEdit((el) => {
              if (key === "fontSize") el.style.fontSize = `${v}px`;
              else {
                if (key === "left" || key === "top") ensurePositioned(el);
                el.style[key] = `${v}px`;
              }
            });
          }}
          style={inputStyle}
        />
      </label>
    );

    return (
      <div style={{ display: "flex", flex: 1, minHeight: 0 }} onKeyDown={handleKey} tabIndex={0} aria-label="Trình sửa thiết kế">
        <div ref={containerRef} style={{ flex: 1, minWidth: 0, overflow: "auto", display: "flex", alignItems: "center", justifyContent: "center", backgroundColor: "var(--cs-bg-canvas)" }}>
          <div style={{ width: size.w * scale, height: size.h * scale, flexShrink: 0, boxShadow: "var(--cs-shadow-artboard)" }}>
            <iframe
              key={reloadKey}
              ref={iframeRef}
              title="Artboard"
              srcDoc={srcDoc}
              sandbox="allow-same-origin"
              onLoad={handleLoad}
              style={{ width: size.w, height: size.h, border: "none", transform: `scale(${scale})`, transformOrigin: "0 0", backgroundColor: "var(--cs-canvas-artboard)" }}
            />
          </div>
        </div>

        <aside style={{ width: 240, flexShrink: 0, borderLeft: "1px solid var(--cs-border-subtle)", backgroundColor: "var(--cs-bg-panel)", padding: "var(--cs-space-3)", overflowY: "auto", display: "flex", flexDirection: "column", gap: "var(--cs-space-3)", fontSize: "var(--cs-text-sm)" }}>
          <div>
            <div style={{ fontWeight: 600, marginBottom: 6 }}>Lớp ({layers.length})</div>
            {layers.length === 0 ? (
              <div style={{ color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-xs)" }}>
                Không có phần tử nào gắn <code>data-layer</code>. Nhờ agent đánh dấu các lớp để sửa trực tiếp.
              </div>
            ) : (
              <div role="listbox" aria-label="Danh sách lớp" style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                {layers.map((l) => (
                  <button
                    key={l.id}
                    role="option"
                    aria-selected={l.id === selectedId}
                    onClick={() => select(l.id)}
                    style={{
                      textAlign: "left", padding: "4px 6px", border: "none", borderRadius: 2, cursor: "pointer",
                      backgroundColor: l.id === selectedId ? "var(--cs-selection)" : "transparent",
                      color: "var(--cs-fg-primary)", fontSize: "var(--cs-text-xs)",
                    }}
                  >
                    {l.id} <span style={{ color: "var(--cs-fg-muted)" }}>· {l.tag}</span>
                  </button>
                ))}
              </div>
            )}
          </div>

          {inspector && selectedId ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <span style={{ fontWeight: 600 }}>{selectedId}</span>
                <Badge variant="neutral">Đang chọn</Badge>
              </div>
              {layers.find((l) => l.id === selectedId)?.isText && (
                <label style={{ display: "flex", flexDirection: "column", gap: 2, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                  Nội dung chữ
                  <textarea
                    rows={3}
                    value={inspector.text}
                    onBlur={() => (editing.current = false)}
                    onChange={(e) => {
                      const v = e.target.value;
                      fieldEdit((el) => (el.textContent = v));
                    }}
                    style={{ ...inputStyle, resize: "vertical", fontFamily: "var(--cs-font-ui)" }}
                  />
                </label>
              )}
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6 }}>
                {numField("Trái (px)", "left")}
                {numField("Trên (px)", "top")}
                {numField("Rộng (px)", "width")}
                {numField("Cao (px)", "height")}
                {numField("Cỡ chữ (px)", "fontSize")}
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6 }}>
                <label style={colorLabel}>
                  Màu chữ
                  <input type="color" value={inspector.color || "#000000"} onBlur={() => (editing.current = false)} onChange={(e) => { const v = e.target.value; fieldEdit((el) => (el.style.color = v)); }} />
                </label>
                <label style={colorLabel}>
                  Màu nền
                  <input type="color" value={inspector.background || "#ffffff"} onBlur={() => (editing.current = false)} onChange={(e) => { const v = e.target.value; fieldEdit((el) => (el.style.backgroundColor = v)); }} />
                </label>
              </div>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => onAskAi(selectedId, layerEl(selectedId)?.textContent?.trim().slice(0, 200) ?? "")}
                leftIcon={<Icon name="sparkles" size={13} />}
              >
                Hỏi AI về lớp này
              </Button>
              <div style={{ color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-xs)", lineHeight: 1.5 }}>
                Mũi tên: dịch 1px · Alt + mũi tên: 10px · Tab: lớp kế tiếp · Ctrl+Z / Ctrl+Y: hoàn tác / làm lại · Ctrl+S: lưu
              </div>
            </div>
          ) : (
            <div style={{ color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-xs)" }}>
              Bấm vào một lớp trên artboard hoặc trong danh sách để sửa.
            </div>
          )}
        </aside>
      </div>
    );
  }
);
DesignEditor.displayName = "DesignEditor";

const inputStyle: React.CSSProperties = {
  padding: "4px 6px",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: 2,
  backgroundColor: "var(--cs-bg-app)",
  color: "var(--cs-fg-primary)",
  fontSize: "var(--cs-text-sm)",
};

const colorLabel: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 2,
  fontSize: "var(--cs-text-xs)",
  color: "var(--cs-fg-secondary)",
};
