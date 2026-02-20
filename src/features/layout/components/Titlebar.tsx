import { Component, type ErrorInfo, type ReactNode } from "react";
import { WindowControls } from "tauri-controls";
import { ThemeToggle } from "@/features/theme";

interface ErrorBoundaryState {
  hasError: boolean;
}

class WindowControlsErrorBoundary extends Component<
  { children: ReactNode },
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.warn("WindowControls failed to render:", error, info);
  }

  render() {
    if (this.state.hasError) {
      return null;
    }
    return this.props.children;
  }
}

export function Titlebar() {
  return (
    <header
      className="flex h-10 shrink-0 items-center border-b border-border bg-background px-3"
      data-tauri-drag-region
    >
      <span className="pointer-events-none select-none text-sm font-semibold text-foreground">
        Screenshot Tool
      </span>

      <div className="ml-auto flex items-center gap-1">
        <ThemeToggle />
        <WindowControlsErrorBoundary>
          <WindowControls />
        </WindowControlsErrorBoundary>
      </div>
    </header>
  );
}
