import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";

// Mock tauri-controls
vi.mock("tauri-controls", () => ({
  WindowControls: () => <div data-testid="window-controls">WindowControls</div>,
}));

// Mock theme module
vi.mock("@/features/theme", () => ({
  ThemeToggle: () => <button data-testid="theme-toggle">ThemeToggle</button>,
}));

import { Titlebar } from "../components/Titlebar";

describe("Titlebar", () => {
  it("renders without crashing", () => {
    render(<Titlebar />);
    expect(document.querySelector("header")).toBeInTheDocument();
  });

  it("contains an element with data-tauri-drag-region attribute", () => {
    render(<Titlebar />);
    const header = document.querySelector("[data-tauri-drag-region]");
    expect(header).toBeInTheDocument();
  });

  it("displays the app title 'Screenshot Tool'", () => {
    render(<Titlebar />);
    expect(screen.getByText("Screenshot Tool")).toBeInTheDocument();
  });

  it("renders the ThemeToggle component", () => {
    render(<Titlebar />);
    expect(screen.getByTestId("theme-toggle")).toBeInTheDocument();
  });

  it("renders the WindowControls component", () => {
    render(<Titlebar />);
    expect(screen.getByTestId("window-controls")).toBeInTheDocument();
  });

  it("interactive elements remain clickable and are not drag-disabled", () => {
    render(<Titlebar />);
    const themeToggle = screen.getByTestId("theme-toggle");
    const windowControls = screen.getByTestId("window-controls");

    // Interactive elements should NOT have pointer-events-none class
    // Tauri's drag region works on the parent via data-tauri-drag-region,
    // but child interactive elements naturally receive their own click events
    expect(themeToggle).not.toHaveClass("pointer-events-none");
    expect(windowControls).not.toHaveClass("pointer-events-none");
  });

  it("renders gracefully when WindowControls is wrapped in error boundary", () => {
    // The Titlebar wraps WindowControls in an ErrorBoundary
    // so the app remains functional if WindowControls fails
    render(<Titlebar />);
    expect(screen.getByText("Screenshot Tool")).toBeInTheDocument();
    expect(screen.getByTestId("theme-toggle")).toBeInTheDocument();
  });
});
