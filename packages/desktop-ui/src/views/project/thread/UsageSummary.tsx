import React from "react";
import type { ThreadMessage } from "../../../runtime/chat/turn-reducer";

export type UsageInfo = NonNullable<ThreadMessage["usage"]>;

export interface UsageSummaryProps {
  usage: UsageInfo;
  model?: string;
}

/** Per-turn token breakdown: total, cost, cache hit ratio and cache write (warm-up). */
export const UsageSummary: React.FC<UsageSummaryProps> = ({ usage, model }) => {
  const fmt = (n: number) => n.toLocaleString();
  const promptTokens = usage.inputTokens + usage.cacheReadTokens + usage.cacheWriteTokens;
  const total = promptTokens + usage.outputTokens;
  const hitPct = promptTokens > 0 ? Math.round((usage.cacheReadTokens / promptTokens) * 100) : 0;

  const head: string[] = [];
  if (model) head.push(model);
  if (usage.costUsd != null) head.push(`$${usage.costUsd.toFixed(4)}`);
  if (usage.estimatedCostUsd) head.push(`~$${usage.estimatedCostUsd.toFixed(2)} (ước tính)`);
  if (total > 0) head.push(`${fmt(total)} tokens`);

  const detail: string[] = [];
  if (total > 0) {
    detail.push(`${fmt(usage.inputTokens)} in`, `${fmt(usage.outputTokens)} out`);
    if (usage.cacheReadTokens > 0) detail.push(`cache đọc ${fmt(usage.cacheReadTokens)} (${hitPct}%)`);
    if (usage.cacheWriteTokens > 0) detail.push(`cache ghi ${fmt(usage.cacheWriteTokens)}`);
    if (usage.reasoningTokens > 0) detail.push(`suy luận ${fmt(usage.reasoningTokens)}`);
  }

  const tooltip = [
    `Input (không cache): ${fmt(usage.inputTokens)}`,
    `Cache đọc (hit): ${fmt(usage.cacheReadTokens)}`,
    `Cache ghi (warm-up): ${fmt(usage.cacheWriteTokens)}`,
    `Output: ${fmt(usage.outputTokens)}`,
    `Suy luận: ${fmt(usage.reasoningTokens)}`,
    `Tỉ lệ cache hit: ${hitPct}%`,
  ].join("\n");

  return (
    <div title={tooltip}>
      {head.length > 0 && <div>{head.join(" · ")}</div>}
      {detail.length > 0 && <div style={{ opacity: 0.8 }}>{detail.join(" · ")}</div>}
    </div>
  );
};
