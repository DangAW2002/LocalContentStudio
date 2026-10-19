import React from "react";
import { Icon } from "../../../components/Icon";
import { Button } from "@studio/design-system";
import { renderMarkdown } from "../../../runtime/markdown";

export interface TextEditorViewerProps {
  kind: "markdown" | "text" | "code" | "json";
  textValue: string;
  setTextValue: (val: string) => void;
  setIsDirty: (dirty: boolean) => void;
}

export const TextEditorViewer: React.FC<TextEditorViewerProps> = ({
  kind,
  textValue,
  setTextValue,
  setIsDirty,
}) => {
  const [showMdPreview, setShowMdPreview] = React.useState(false);

  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        height: "100%",
        backgroundColor: "var(--cs-bg-panel)",
        position: "relative",
        overflow: "hidden",
      }}
    >
      {/* Markdown subheader: edit / preview toggle (save state lives in the main header). */}
      {kind === "markdown" && (
        <div
          style={{
            height: 32,
            padding: "0 var(--cs-space-3)",
            display: "flex",
            alignItems: "center",
            justifyContent: "flex-end",
            borderBottom: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-raised)",
            fontSize: "var(--cs-text-xs)",
          }}
        >
          <Button
            size="sm"
            variant={showMdPreview ? "secondary" : "quiet"}
            onClick={() => setShowMdPreview(!showMdPreview)}
            leftIcon={<Icon name="eye" size={12} />}
          >
            {showMdPreview ? "Chỉnh sửa" : "Xem thử Markdown"}
          </Button>
        </div>
      )}

      {/* Editor Body */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden", position: "relative" }}>
        {kind === "markdown" && showMdPreview ? (
          <div
            style={{
              flex: 1,
              padding: "var(--cs-space-4) var(--cs-space-6)",
              overflowY: "auto",
              overflowX: "hidden",
              maxWidth: 820,
              margin: "0 auto",
              width: "100%",
              fontSize: "var(--cs-text-sm)",
              fontFamily: "var(--cs-font-ui)",
              boxSizing: "border-box",
            }}
          >
            <div className="cs-md">{renderMarkdown(textValue)}</div>
          </div>
        ) : (
          <textarea
            value={textValue}
            onChange={(e) => {
              setTextValue(e.target.value);
              setIsDirty(true);
            }}
            spellCheck={false}
            style={{
              flex: 1,
              width: "100%",
              height: "100%",
              padding: "var(--cs-space-3)",
              margin: 0,
              border: "none",
              outline: "none",
              resize: "none",
              backgroundColor: "var(--cs-bg-panel)",
              color: "var(--cs-fg-primary)",
              fontFamily: "var(--cs-font-mono)",
              fontSize: "var(--cs-text-xs)",
              lineHeight: 1.6,
              whiteSpace: "pre",
              boxSizing: "border-box",
            }}
          />
        )}
      </div>
    </div>
  );
};
