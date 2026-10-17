import React from "react";
import { Button, Callout } from "@studio/design-system";

interface PaneBoundaryProps {
  /** Vietnamese pane name shown in the error message. */
  name: string;
  children: React.ReactNode;
}

interface PaneBoundaryState {
  error: Error | null;
}

/** Keeps a crash inside one pane from blanking the whole window. */
export class PaneBoundary extends React.Component<PaneBoundaryProps, PaneBoundaryState> {
  state: PaneBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): PaneBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error(`Pane "${this.props.name}" crashed`, error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div style={{ padding: "var(--cs-space-4)" }}>
        <Callout variant="danger" title={`Khung ${this.props.name} gặp lỗi`}>
          <div style={{ marginBottom: "var(--cs-space-2)" }}>{this.state.error.message}</div>
          <Button size="sm" variant="secondary" onClick={() => this.setState({ error: null })}>
            Thử lại
          </Button>
        </Callout>
      </div>
    );
  }
}
