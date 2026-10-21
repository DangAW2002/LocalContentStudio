import React from "react";
import { Spinner } from "@studio/design-system";

const GraphFlowLazy = React.lazy(() =>
  import("./GraphFlow").then((m) => ({ default: m.GraphFlow }))
);

export interface GraphPaneProps {
  projectSlug: string;
  onSendToAgent?: (refs: string[]) => void;
  onOpenArtifact?: (artifactId: string) => void;
}

export const GraphPane: React.FC<GraphPaneProps> = (props) => {
  return (
    <React.Suspense
      fallback={
        <div
          style={{
            height: "100%",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            gap: 8,
            color: "var(--cs-fg-muted)",
          }}
        >
          <Spinner size={16} />
          <span>Đang tải Sơ đồ...</span>
        </div>
      }
    >
      <GraphFlowLazy {...props} />
    </React.Suspense>
  );
};
