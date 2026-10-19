import React from "react";
import { Button } from "@studio/design-system";
import type { BrandSpec } from "../../utils/brandYaml";

export interface BrandVoiceSectionProps {
  brandSpec: BrandSpec;
  setBrandSpec: (spec: BrandSpec) => void;
}

export const BrandVoiceSection: React.FC<BrandVoiceSectionProps> = ({
  brandSpec,
  setBrandSpec,
}) => {
  const [newPreferWord, setNewPreferWord] = React.useState("");
  const [newAvoidWord, setNewAvoidWord] = React.useState("");
  const [newRule, setNewRule] = React.useState("");

  return (
    <>
      {/* 5. Voice & Words */}
      <div style={cardSectionStyle}>
        <div style={sectionTitleStyle}>5. GIỌNG VĂN & TỪ NGỮ (VOICE WORDS)</div>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
          {/* Prefer words */}
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-success)" }}>
              ✓ Từ ngữ ưu tiên sử dụng ({brandSpec.voice_words.prefer.length})
            </span>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
              {brandSpec.voice_words.prefer.map((w, idx) => (
                <span
                  key={idx}
                  style={{
                    padding: "2px 8px",
                    backgroundColor: "rgba(16, 185, 129, 0.15)",
                    border: "1px solid rgba(16, 185, 129, 0.3)",
                    borderRadius: "var(--cs-radius-control)",
                    fontSize: 11,
                    color: "var(--cs-fg-primary)",
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 4,
                  }}
                >
                  {w}
                  <button
                    type="button"
                    onClick={() => {
                      const next = [...brandSpec.voice_words.prefer];
                      next.splice(idx, 1);
                      setBrandSpec({ ...brandSpec, voice_words: { ...brandSpec.voice_words, prefer: next } });
                    }}
                    style={{ background: "none", border: "none", cursor: "pointer", color: "var(--cs-fg-muted)", padding: 0 }}
                  >
                    ×
                  </button>
                </span>
              ))}
            </div>
            <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
              <input
                type="text"
                placeholder="Thêm từ ưu tiên..."
                value={newPreferWord}
                onChange={(e) => setNewPreferWord(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newPreferWord.trim()) {
                    e.preventDefault();
                    setBrandSpec({
                      ...brandSpec,
                      voice_words: { ...brandSpec.voice_words, prefer: [...brandSpec.voice_words.prefer, newPreferWord.trim()] },
                    });
                    setNewPreferWord("");
                  }
                }}
                style={{ ...inputStyle, flex: 1 }}
              />
              <Button
                size="sm"
                variant="secondary"
                onClick={() => {
                  if (newPreferWord.trim()) {
                    setBrandSpec({
                      ...brandSpec,
                      voice_words: { ...brandSpec.voice_words, prefer: [...brandSpec.voice_words.prefer, newPreferWord.trim()] },
                    });
                    setNewPreferWord("");
                  }
                }}
              >
                Thêm
              </Button>
            </div>
          </div>

          {/* Avoid words */}
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-danger)" }}>
              ⚠ Từ ngữ cần tránh ({brandSpec.voice_words.avoid.length})
            </span>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
              {brandSpec.voice_words.avoid.map((w, idx) => (
                <span
                  key={idx}
                  style={{
                    padding: "2px 8px",
                    backgroundColor: "rgba(239, 68, 68, 0.15)",
                    border: "1px solid rgba(239, 68, 68, 0.3)",
                    borderRadius: "var(--cs-radius-control)",
                    fontSize: 11,
                    color: "var(--cs-fg-primary)",
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 4,
                  }}
                >
                  {w}
                  <button
                    type="button"
                    onClick={() => {
                      const next = [...brandSpec.voice_words.avoid];
                      next.splice(idx, 1);
                      setBrandSpec({ ...brandSpec, voice_words: { ...brandSpec.voice_words, avoid: next } });
                    }}
                    style={{ background: "none", border: "none", cursor: "pointer", color: "var(--cs-fg-muted)", padding: 0 }}
                  >
                    ×
                  </button>
                </span>
              ))}
            </div>
            <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
              <input
                type="text"
                placeholder="Thêm từ cần tránh..."
                value={newAvoidWord}
                onChange={(e) => setNewAvoidWord(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newAvoidWord.trim()) {
                    e.preventDefault();
                    setBrandSpec({
                      ...brandSpec,
                      voice_words: { ...brandSpec.voice_words, avoid: [...brandSpec.voice_words.avoid, newAvoidWord.trim()] },
                    });
                    setNewAvoidWord("");
                  }
                }}
                style={{ ...inputStyle, flex: 1 }}
              />
              <Button
                size="sm"
                variant="secondary"
                onClick={() => {
                  if (newAvoidWord.trim()) {
                    setBrandSpec({
                      ...brandSpec,
                      voice_words: { ...brandSpec.voice_words, avoid: [...brandSpec.voice_words.avoid, newAvoidWord.trim()] },
                    });
                    setNewAvoidWord("");
                  }
                }}
              >
                Thêm
              </Button>
            </div>
          </div>
        </div>
      </div>

      {/* 6. Brand Rules */}
      <div style={cardSectionStyle}>
        <div style={sectionTitleStyle}>6. NGUYÊN TẮC THIẾT KẾ & QUY CHUẨN (RULES)</div>
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          {brandSpec.rules.map((rule, idx) => (
            <div
              key={idx}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                padding: "6px 10px",
                backgroundColor: "var(--cs-bg-raised)",
                border: "1px solid var(--cs-border-subtle)",
                borderRadius: "var(--cs-radius-control)",
                fontSize: "var(--cs-text-xs)",
              }}
            >
              <span>{rule}</span>
              <button
                type="button"
                onClick={() => {
                  const next = [...brandSpec.rules];
                  next.splice(idx, 1);
                  setBrandSpec({ ...brandSpec, rules: next });
                }}
                style={{ background: "none", border: "none", color: "var(--cs-fg-muted)", cursor: "pointer" }}
              >
                ×
              </button>
            </div>
          ))}

          <div style={{ display: "flex", gap: 4, marginTop: 4 }}>
            <input
              type="text"
              placeholder="Thêm nguyên tắc thiết kế mới..."
              value={newRule}
              onChange={(e) => setNewRule(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && newRule.trim()) {
                  e.preventDefault();
                  setBrandSpec({ ...brandSpec, rules: [...brandSpec.rules, newRule.trim()] });
                  setNewRule("");
                }
              }}
              style={{ ...inputStyle, flex: 1 }}
            />
            <Button
              size="sm"
              variant="secondary"
              onClick={() => {
                if (newRule.trim()) {
                  setBrandSpec({ ...brandSpec, rules: [...brandSpec.rules, newRule.trim()] });
                  setNewRule("");
                }
              }}
            >
              Thêm nguyên tắc
            </Button>
          </div>
        </div>
      </div>
    </>
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

const inputStyle: React.CSSProperties = {
  padding: "6px 10px",
  backgroundColor: "var(--cs-bg-app)",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
  color: "var(--cs-fg-primary)",
  fontSize: "var(--cs-text-xs)",
  outline: "none",
};
