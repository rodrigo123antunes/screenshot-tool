import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";

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

  it("displays the app title 'Screenshot Tool'", () => {
    render(<Titlebar />);
    expect(screen.getByText("Screenshot Tool")).toBeInTheDocument();
  });

  it("renders the ThemeToggle component", () => {
    render(<Titlebar />);
    expect(screen.getByTestId("theme-toggle")).toBeInTheDocument();
  });
});
