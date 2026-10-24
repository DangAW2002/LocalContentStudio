import React, { useState } from "react";
import { Badge, Button } from "@studio/design-system";
import { Icon } from "../../../components/Icon";
import type {
  AskUserQuestionInput,
  AskUserQuestionItem,
  AskUserQuestionOption,
} from "../../../api/types";

export interface AskUserQuestionCardProps {
  requestId: string;
  tool: string;
  input: unknown;
  resolved?: boolean;
  answers?: Record<string, string | string[]>;
  onRespond?: (answers: Record<string, string | string[]>) => void;
}

/** Parses input into normalized AskUserQuestionInput */
export function parseQuestionInput(raw: unknown): AskUserQuestionInput {
  if (!raw) return { questions: [] };
  const obj =
    typeof raw === "string"
      ? (() => {
          try {
            return JSON.parse(raw);
          } catch {
            return {};
          }
        })()
      : (raw as any);

  const questions: AskUserQuestionItem[] = Array.isArray(obj?.questions)
    ? obj.questions.map((q: any) => ({
        question: String(q?.question || ""),
        header: q?.header ? String(q.header) : undefined,
        description: q?.description ? String(q.description) : undefined,
        multiSelect: Boolean(q?.multiSelect),
        options: Array.isArray(q?.options)
          ? q.options.map((opt: any): AskUserQuestionOption =>
              typeof opt === "string"
                ? { label: opt }
                : {
                    label: String(opt?.label || ""),
                    description: opt?.description ? String(opt.description) : undefined,
                    preview: opt?.preview ? String(opt.preview) : undefined,
                  }
            )
          : [],
      }))
    : [];

  return {
    title: obj?.title ? String(obj.title) : undefined,
    questions,
  };
}

/** Extracts clean display label and whether this option is recommended */
export function parseOptionLabel(rawLabel: string): { displayLabel: string; isRecommended: boolean } {
  const recRegex = /\s*\((?:Recommended|Khuyến\s*nghị)\)\s*$/i;
  if (recRegex.test(rawLabel)) {
    return {
      displayLabel: rawLabel.replace(recRegex, "").trim(),
      isRecommended: true,
    };
  }
  return {
    displayLabel: rawLabel,
    isRecommended: false,
  };
}

export const AskUserQuestionCard: React.FC<AskUserQuestionCardProps> = ({
  requestId,
  input,
  resolved = false,
  answers: propAnswers,
  onRespond,
}) => {
  const parsed = parseQuestionInput(input);
  const questions = parsed.questions;

  // Track selection per question index
  const [selectedOptions, setSelectedOptions] = useState<Record<number, string[]>>({});
  const [otherActive, setOtherActive] = useState<Record<number, boolean>>({});
  const [otherText, setOtherText] = useState<Record<number, string>>({});
  const [submitting, setSubmitting] = useState(false);

  // If resolved and propAnswers exists, use propAnswers for display
  const effectiveAnswers = propAnswers || {};

  const handleToggleOption = (qIdx: number, rawLabel: string, isMulti: boolean) => {
    if (resolved || submitting) return;

    if (isMulti) {
      setSelectedOptions((prev) => {
        const curr = prev[qIdx] || [];
        const next = curr.includes(rawLabel)
          ? curr.filter((l) => l !== rawLabel)
          : [...curr, rawLabel];
        return { ...prev, [qIdx]: next };
      });
    } else {
      setSelectedOptions((prev) => ({ ...prev, [qIdx]: [rawLabel] }));
      setOtherActive((prev) => ({ ...prev, [qIdx]: false }));
    }
  };

  const handleToggleOther = (qIdx: number, isMulti: boolean) => {
    if (resolved || submitting) return;

    if (isMulti) {
      setOtherActive((prev) => ({ ...prev, [qIdx]: !prev[qIdx] }));
    } else {
      setOtherActive((prev) => ({ ...prev, [qIdx]: true }));
      setSelectedOptions((prev) => ({ ...prev, [qIdx]: [] }));
    }
  };

  const handleOtherTextChange = (qIdx: number, text: string, isMulti: boolean) => {
    if (resolved || submitting) return;

    setOtherText((prev) => ({ ...prev, [qIdx]: text }));
    setOtherActive((prev) => ({ ...prev, [qIdx]: true }));
    if (!isMulti) {
      setSelectedOptions((prev) => ({ ...prev, [qIdx]: [] }));
    }
  };

  // Determine if all questions have at least one response
  const canSubmit =
    !resolved &&
    !submitting &&
    questions.length > 0 &&
    questions.every((q, idx) => {
      const selected = selectedOptions[idx] || [];
      const hasOther = Boolean(otherActive[idx] && otherText[idx]?.trim());
      return selected.length > 0 || hasOther;
    });

  const handleSubmit = () => {
    if (!canSubmit || !onRespond) return;
    setSubmitting(true);

    const answersMap: Record<string, string | string[]> = {};
    questions.forEach((q, idx) => {
      const selected = selectedOptions[idx] || [];
      const hasOther = Boolean(otherActive[idx] && otherText[idx]?.trim());
      if (q.multiSelect) {
        const list = [...selected];
        if (hasOther) {
          list.push(otherText[idx].trim());
        }
        answersMap[q.question] = list;
      } else {
        if (hasOther) {
          answersMap[q.question] = otherText[idx].trim();
        } else if (selected.length > 0) {
          answersMap[q.question] = selected[0];
        }
      }
    });

    onRespond(answersMap);
  };

  return (
    <div
      role="region"
      aria-label="Thẻ câu hỏi của Agent"
      style={{
        marginTop: 8,
        padding: "var(--cs-space-3)",
        border: resolved ? "1px solid var(--cs-border-subtle)" : "1px solid var(--cs-accent)",
        backgroundColor: "var(--cs-bg-raised)",
        borderRadius: "var(--cs-radius-control)",
        display: "flex",
        flexDirection: "column",
        gap: 12,
        color: "var(--cs-fg-primary)",
      }}
    >
      {/* Card Header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          borderBottom: "1px solid var(--cs-border-subtle)",
          paddingBottom: 8,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Icon name="message-square" size={16} color="var(--cs-accent)" />
          <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
            {parsed.title || "Câu hỏi từ Agent"}
          </span>
        </div>
        {resolved ? (
          <Badge variant="success">Đã trả lời</Badge>
        ) : (
          <Badge variant="warning">Cần lựa chọn</Badge>
        )}
      </div>

      {/* Questions List */}
      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        {questions.map((q, qIdx) => {
          const selected = selectedOptions[qIdx] || [];
          const isOther = Boolean(otherActive[qIdx]);
          const currentOtherText = otherText[qIdx] || "";
          const resolvedAnswer = effectiveAnswers[q.question];

          return (
            <div
              key={qIdx}
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 8,
                padding: "8px 10px",
                backgroundColor: "var(--cs-bg-panel)",
                borderRadius: "var(--cs-radius-control)",
                border: "1px solid var(--cs-border-subtle)",
              }}
            >
              {/* Question Header & Category */}
              <div>
                {q.header && (
                  <div
                    style={{
                      fontSize: "10px",
                      textTransform: "uppercase",
                      letterSpacing: "0.5px",
                      fontWeight: 700,
                      color: "var(--cs-accent)",
                      marginBottom: 2,
                    }}
                  >
                    {q.header}
                  </div>
                )}
                <div
                  style={{
                    fontWeight: 600,
                    fontSize: "var(--cs-text-sm)",
                    color: "var(--cs-fg-primary)",
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    flexWrap: "wrap",
                  }}
                >
                  <span>{q.question}</span>
                  {q.multiSelect && !resolved && (
                    <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontWeight: 400 }}>
                      (Có thể chọn nhiều phương án)
                    </span>
                  )}
                </div>
                {q.description && (
                  <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", marginTop: 2 }}>
                    {q.description}
                  </div>
                )}
              </div>

              {/* Resolved View for this Question */}
              {resolved ? (
                <div
                  style={{
                    padding: "6px 10px",
                    backgroundColor: "var(--cs-bg-raised)",
                    borderRadius: "var(--cs-radius-control)",
                    fontSize: "var(--cs-text-xs)",
                    display: "flex",
                    flexDirection: "column",
                    gap: 4,
                  }}
                >
                  <span style={{ color: "var(--cs-fg-muted)", fontWeight: 600 }}>Câu trả lời của bạn:</span>
                  <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
                    {Array.isArray(resolvedAnswer) ? (
                      resolvedAnswer.map((ans, aIdx) => (
                        <Badge key={aIdx} variant="info">
                          {ans}
                        </Badge>
                      ))
                    ) : resolvedAnswer ? (
                      <Badge variant="info">{String(resolvedAnswer)}</Badge>
                    ) : (
                      <span style={{ color: "var(--cs-fg-muted)", fontStyle: "italic" }}>
                        (Không có lựa chọn)
                      </span>
                    )}
                  </div>
                </div>
              ) : (
                /* Interactive Options */
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {q.options?.map((opt, oIdx) => {
                    const isSelected = selected.includes(opt.label);
                    const { displayLabel, isRecommended } = parseOptionLabel(opt.label);

                    return (
                      <div
                        key={oIdx}
                        role={q.multiSelect ? "checkbox" : "radio"}
                        aria-checked={isSelected}
                        tabIndex={0}
                        onClick={() => handleToggleOption(qIdx, opt.label, Boolean(q.multiSelect))}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            handleToggleOption(qIdx, opt.label, Boolean(q.multiSelect));
                          }
                        }}
                        style={{
                          display: "flex",
                          flexDirection: "column",
                          gap: 2,
                          padding: "6px 10px",
                          borderRadius: "var(--cs-radius-control)",
                          border: isSelected
                            ? "1px solid var(--cs-accent)"
                            : "1px solid var(--cs-border-subtle)",
                          backgroundColor: isSelected
                            ? "var(--cs-bg-raised)"
                            : "var(--cs-bg-panel)",
                          cursor: "pointer",
                          userSelect: "none",
                          outline: "none",
                          transition: "border-color 0.15s, background-color 0.15s",
                        }}
                      >
                        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
                          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                            <div
                              style={{
                                width: 14,
                                height: 14,
                                borderRadius: q.multiSelect ? 3 : "50%",
                                border: isSelected
                                  ? "4px solid var(--cs-accent)"
                                  : "1px solid var(--cs-border-strong)",
                                backgroundColor: isSelected ? "var(--cs-bg-panel)" : "transparent",
                                boxSizing: "border-box",
                                flexShrink: 0,
                              }}
                            />
                            <span
                              style={{
                                fontSize: "var(--cs-text-xs)",
                                fontWeight: isSelected ? 600 : 500,
                                color: isSelected ? "var(--cs-fg-primary)" : "var(--cs-fg-secondary)",
                              }}
                            >
                              {displayLabel}
                            </span>
                          </div>
                          {isRecommended && (
                            <Badge variant="info">Khuyến nghị</Badge>
                          )}
                        </div>
                        {opt.description && (
                          <div
                            style={{
                              paddingLeft: 22,
                              fontSize: "11px",
                              color: "var(--cs-fg-muted)",
                            }}
                          >
                            {opt.description}
                          </div>
                        )}
                      </div>
                    );
                  })}

                  {/* Option "Khác" (Custom Input) */}
                  <div
                    style={{
                      display: "flex",
                      flexDirection: "column",
                      gap: 4,
                      padding: "6px 10px",
                      borderRadius: "var(--cs-radius-control)",
                      border: isOther
                        ? "1px solid var(--cs-accent)"
                        : "1px solid var(--cs-border-subtle)",
                      backgroundColor: isOther
                        ? "var(--cs-bg-raised)"
                        : "var(--cs-bg-panel)",
                    }}
                  >
                    <div
                      role={q.multiSelect ? "checkbox" : "radio"}
                      aria-checked={isOther}
                      tabIndex={0}
                      onClick={() => handleToggleOther(qIdx, Boolean(q.multiSelect))}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          handleToggleOther(qIdx, Boolean(q.multiSelect));
                        }
                      }}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 8,
                        cursor: "pointer",
                        userSelect: "none",
                        outline: "none",
                      }}
                    >
                      <div
                        style={{
                          width: 14,
                          height: 14,
                          borderRadius: q.multiSelect ? 3 : "50%",
                          border: isOther
                            ? "4px solid var(--cs-accent)"
                            : "1px solid var(--cs-border-strong)",
                          backgroundColor: isOther ? "var(--cs-bg-panel)" : "transparent",
                          boxSizing: "border-box",
                          flexShrink: 0,
                        }}
                      />
                      <span
                        style={{
                          fontSize: "var(--cs-text-xs)",
                          fontWeight: isOther ? 600 : 500,
                          color: isOther ? "var(--cs-fg-primary)" : "var(--cs-fg-secondary)",
                        }}
                      >
                        Khác (nhập phương án của bạn)
                      </span>
                    </div>

                    {isOther && (
                      <div style={{ paddingLeft: 22, paddingTop: 4 }}>
                        <input
                          type="text"
                          aria-label={`Phương án khác cho câu hỏi: ${q.question}`}
                          value={currentOtherText}
                          placeholder="Nhập nội dung lựa chọn tự do..."
                          onChange={(e) =>
                            handleOtherTextChange(qIdx, e.target.value, Boolean(q.multiSelect))
                          }
                          autoFocus
                          style={{
                            width: "100%",
                            padding: "4px 8px",
                            fontSize: "var(--cs-text-xs)",
                            backgroundColor: "var(--cs-bg-panel)",
                            color: "var(--cs-fg-primary)",
                            border: "1px solid var(--cs-border-strong)",
                            borderRadius: "var(--cs-radius-control)",
                            outline: "none",
                            boxSizing: "border-box",
                          }}
                        />
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Action Footer */}
      {!resolved && (
        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 4 }}>
          <Button
            size="sm"
            variant="primary"
            onClick={handleSubmit}
            disabled={!canSubmit || submitting}
          >
            {submitting ? "Đang gửi..." : "Gửi câu trả lời"}
          </Button>
        </div>
      )}
    </div>
  );
};
