import React from "react";
import { Button, Badge, Tabs } from "@studio/design-system";
import { Icon } from "../../components/Icon";
import type { ScopeItem } from "../../api/types";
import { renderMarkdown } from "../../runtime/markdown";
import { studioApi } from "../../api/client";

export interface PlaybookDetailProps {
  selectedItem: ScopeItem;
  rawText: string | null;
  onSave?: (fullText: string) => Promise<void>;
  onRevertOverride?: () => Promise<void>;
}

export const PlaybookDetail: React.FC<PlaybookDetailProps> = ({
  selectedItem,
  rawText,
  onSave,
  onRevertOverride,
}) => {
  const [tab, setTab] = React.useState<"edit" | "preview" | "raw">("edit");
  const [name, setName] = React.useState<string>("");
  const [description, setDescription] = React.useState<string>("");
  const [body, setBody] = React.useState<string>("");
  const [rawMarkdown, setRawMarkdown] = React.useState<string>(rawText || "");
  const [saving, setSaving] = React.useState(false);
  const [saveSuccess, setSaveSuccess] = React.useState(false);
  const [reverting, setReverting] = React.useState(false);
  const [errorMsg, setErrorMsg] = React.useState<string | null>(null);
  const [validationErrors, setValidationErrors] = React.useState<Array<{ line: number; column: number; message: string }>>([]);

  // Parse frontmatter and body from markdown
  React.useEffect(() => {
    const text = rawText || "";
    setRawMarkdown(text);
    setErrorMsg(null);
    setValidationErrors([]);

    const trimmed = text.trimStart();
    if (trimmed.startsWith("---")) {
      const afterFirst = trimmed.slice(3);
      const endIdx = afterFirst.indexOf("\n---");
      if (endIdx >= 0) {
        const fmStr = afterFirst.slice(0, endIdx);
        const bodyStr = afterFirst.slice(endIdx + 4).replace(/^[\r\n]+/, "");
        setBody(bodyStr);

        // Simple frontmatter extractor
        let parsedName = selectedItem.name || "";
        let parsedDesc = selectedItem.summary || "";
        for (const line of fmStr.split(/\r?\n/)) {
          const t = line.trim();
          if (t.startsWith("name:")) {
            parsedName = t.slice(5).trim().replace(/^["']|["']$/g, "");
          } else if (t.startsWith("description:")) {
            parsedDesc = t.slice(12).trim().replace(/^["']|["']$/g, "");
          }
        }
        setName(parsedName);
        setDescription(parsedDesc);
        return;
      }
    }

    setName(selectedItem.name || "");
    setDescription(selectedItem.summary || "");
    setBody(text);
  }, [rawText, selectedItem]);

  const composeFullMarkdown = (): string => {
    if (tab === "raw") return rawMarkdown;
    return `---
name: "${name.replace(/"/g, '\\"')}"
description: "${description.replace(/"/g, '\\"')}"
kind: playbook
artifacts: []
steps: []
checkers: []
ask_user_at: []
recipes: []
---

${body}
`;
  };

  const handleTabChange = (nextTab: string) => {
    if (nextTab === "raw" && tab !== "raw") {
      setRawMarkdown(composeFullMarkdown());
    }
    setTab(nextTab as any);
  };

  const handleSave = async () => {
    const textToSave = composeFullMarkdown();
    setSaving(true);
    setErrorMsg(null);
    setValidationErrors([]);

    try {
      const val = await studioApi.scopeValidate("playbooks", textToSave);
      if (!val.valid) {
        setValidationErrors(val.errors);
        setErrorMsg("Cấu hình quy trình chưa đúng định dạng frontmatter.");
        setSaving(false);
        return;
      }

      if (onSave) {
        await onSave(textToSave);
      } else {
        if (selectedItem.layer !== "workspace") {
          await studioApi.scopeOverride("playbooks", selectedItem.id);
        }
        await studioApi.writeFile({
          path: `playbooks/${selectedItem.id}.md`,
          text: textToSave,
        });
      }

      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 3000);
    } catch (err: any) {
      setErrorMsg(err?.message || "Lỗi khi lưu quy trình.");
    } finally {
      setSaving(false);
    }
  };

  const handleRevert = async () => {
    if (!window.confirm(`Bạn có chắc muốn bỏ bản ghi đè của quy trình '${selectedItem.name}' để quay về bản kế thừa?`)) {
      return;
    }
    setReverting(true);
    setErrorMsg(null);
    try {
      if (onRevertOverride) {
        await onRevertOverride();
      } else {
        await studioApi.scopeRemoveOverride("playbooks", selectedItem.id);
      }
    } catch (err: any) {
      setErrorMsg(err?.message || "Lỗi khi bỏ bản ghi đè.");
    } finally {
      setReverting(false);
    }
  };

  const isInherited = selectedItem.layer !== "workspace";
  const isOverriding = Boolean(selectedItem.isOverride);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
      {/* Top Action Bar */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          backgroundColor: "var(--cs-bg-raised)",
          padding: "var(--cs-space-3) var(--cs-space-4)",
          borderRadius: "var(--cs-radius-control)",
          border: "1px solid var(--cs-border-subtle)",
        }}
      >
        <Tabs
          size="sm"
          items={[
            { id: "edit", label: "Biểu mẫu & Hướng dẫn" },
            { id: "preview", label: "Xem trước (Markdown)" },
            { id: "raw", label: "Toàn bộ tệp thô" },
          ]}
          activeId={tab}
          onChange={handleTabChange}
        />

        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {isOverriding && (
            <Button
              size="sm"
              variant="secondary"
              busy={reverting}
              busyText="Đang khôi phục..."
              onClick={handleRevert}
              leftIcon={<Icon name="undo" size={13} />}
              title="Xoá bản ghi đè ở workspace và quay lại bản gốc từ App/Org"
            >
              Bỏ bản ghi đè
            </Button>
          )}

          {saveSuccess && <Badge variant="success">✓ Đã lưu thành công</Badge>}

          <Button
            size="sm"
            variant="primary"
            busy={saving}
            busyText="Đang lưu..."
            onClick={handleSave}
            leftIcon={<Icon name="check" size={13} />}
            title={isInherited ? "Quy trình kế thừa chỉ đọc — lưu sẽ tạo bản ghi đè trong workspace này" : undefined}
          >
            {isInherited ? "Lưu bản ghi đè" : "Lưu quy trình"}
          </Button>
        </div>
      </div>

      {errorMsg && (
        <div
          style={{
            padding: "8px 12px",
            backgroundColor: "rgba(239, 68, 68, 0.1)",
            border: "1px solid var(--cs-danger)",
            borderRadius: "var(--cs-radius-control)",
            color: "var(--cs-danger)",
            fontSize: "var(--cs-text-xs)",
          }}
        >
          {errorMsg}
        </div>
      )}

      {validationErrors.length > 0 && (
        <div
          style={{
            padding: "8px 12px",
            backgroundColor: "rgba(239, 68, 68, 0.08)",
            border: "1px solid var(--cs-danger)",
            borderRadius: "var(--cs-radius-control)",
            color: "var(--cs-danger)",
            fontSize: "var(--cs-text-xs)",
            display: "flex",
            flexDirection: "column",
            gap: 4,
          }}
        >
          <span style={{ fontWeight: 700 }}>Phát hiện lỗi định dạng:</span>
          {validationErrors.map((err, idx) => (
            <div key={idx} style={{ fontFamily: "var(--cs-font-mono)" }}>
              • Dòng {err.line}, Cột {err.column}: {err.message}
            </div>
          ))}
        </div>
      )}

      {tab === "edit" ? (
        <>
          {/* Frontmatter Metadata Form */}
          <div style={cardSectionStyle}>
            <div style={sectionTitleStyle}>1. THÔNG TIN QUY TRÌNH (FRONTMATTER)</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
              <label style={fieldLabelStyle}>
                <span>Tên quy trình</span>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  style={inputStyle}
                  placeholder="Video ngắn, Carousel, Bài viết..."
                />
              </label>

              <label style={fieldLabelStyle}>
                <span>Mô tả ngắn gọn</span>
                <input
                  type="text"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  style={inputStyle}
                  placeholder="Mô tả các bước thực hiện..."
                />
              </label>
            </div>
          </div>

          {/* Markdown Instructions Editor */}
          <div style={cardSectionStyle}>
            <div style={sectionTitleStyle}>2. HƯỚNG DẪN THỰC HIỆN (MARKDOWN)</div>
            <textarea
              value={body}
              onChange={(e) => setBody(e.target.value)}
              rows={16}
              style={{
                width: "100%",
                padding: "var(--cs-space-3)",
                backgroundColor: "var(--cs-bg-raised)",
                border: "1px solid var(--cs-border-subtle)",
                borderRadius: "var(--cs-radius-control)",
                fontFamily: "var(--cs-font-mono)",
                fontSize: 12,
                lineHeight: 1.6,
                resize: "vertical",
                color: "var(--cs-fg-primary)",
                boxSizing: "border-box",
              }}
              placeholder="# Hướng dẫn quy trình..."
            />
          </div>
        </>
      ) : tab === "preview" ? (
        /* Markdown Preview with renderMarkdown */
        <div style={cardSectionStyle}>
          <div style={sectionTitleStyle}>XEM TRƯỚC NỘI DUNG QUY TRÌNH</div>
          <div
            style={{
              padding: "var(--cs-space-4)",
              backgroundColor: "var(--cs-bg-raised)",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              minHeight: 200,
            }}
          >
            <h2 style={{ margin: "0 0 6px 0", fontSize: "var(--cs-text-lg)", color: "var(--cs-fg-primary)" }}>
              {name || "Chưa đặt tên quy trình"}
            </h2>
            {description && (
              <p style={{ margin: "0 0 16px 0", fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
                {description}
              </p>
            )}
            <hr style={{ border: "none", borderTop: "1px solid var(--cs-border-subtle)", margin: "12px 0 16px 0" }} />
            <div className="markdown-body">
              {renderMarkdown(body || "*(Chưa có nội dung hướng dẫn)*")}
            </div>
          </div>
        </div>
      ) : (
        /* Raw File Editor */
        <div style={cardSectionStyle}>
          <div style={sectionTitleStyle}>TOÀN BỘ TỆP MARKDOWN (BAO GỒM FRONTMATTER)</div>
          <textarea
            value={rawMarkdown}
            onChange={(e) => setRawMarkdown(e.target.value)}
            rows={20}
            style={{
              width: "100%",
              padding: "var(--cs-space-3)",
              backgroundColor: "var(--cs-bg-raised)",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              fontFamily: "var(--cs-font-mono)",
              fontSize: 12,
              lineHeight: 1.5,
              resize: "vertical",
              color: "var(--cs-fg-primary)",
              boxSizing: "border-box",
            }}
          />
        </div>
      )}
    </div>
  );
};

const cardSectionStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 8,
  padding: "var(--cs-space-4)",
  backgroundColor: "var(--cs-bg-panel)",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
};

const sectionTitleStyle: React.CSSProperties = {
  fontSize: "var(--cs-text-xs)",
  fontWeight: 700,
  color: "var(--cs-fg-secondary)",
  letterSpacing: 0.5,
};

const fieldLabelStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 4,
  fontSize: "var(--cs-text-xs)",
  fontWeight: 500,
  color: "var(--cs-fg-secondary)",
};

const inputStyle: React.CSSProperties = {
  padding: "6px 8px",
  backgroundColor: "var(--cs-bg-raised)",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
  color: "var(--cs-fg-primary)",
  fontSize: "var(--cs-text-xs)",
  outline: "none",
};
