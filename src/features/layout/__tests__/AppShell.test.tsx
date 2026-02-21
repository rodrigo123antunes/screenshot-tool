import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";

// Mock theme module
vi.mock("@/features/theme", () => ({
  ThemeToggle: () => <button data-testid="theme-toggle">ThemeToggle</button>,
}));

import { AppShell } from "../components/AppShell";

describe("AppShell", () => {
  it("renders without crashing", () => {
    render(<AppShell />);
    expect(document.querySelector("div")).toBeInTheDocument();
  });

  it("contains the Titlebar component", () => {
    render(<AppShell />);
    expect(document.querySelector("header")).toBeInTheDocument();
    expect(screen.getByText("Screenshot Tool")).toBeInTheDocument();
  });

  it("contains the ContentArea component", () => {
    render(<AppShell />);
    expect(document.querySelector("main")).toBeInTheDocument();
  });

  it("passes children through to ContentArea", () => {
    render(
      <AppShell>
        <p data-testid="shell-child">App Content</p>
      </AppShell>,
    );

    expect(screen.getByTestId("shell-child")).toBeInTheDocument();
    expect(screen.getByTestId("shell-child")).toHaveTextContent("App Content");
    // Child should be inside the main (ContentArea)
    const main = document.querySelector("main");
    expect(main?.querySelector("[data-testid='shell-child']")).toBeInTheDocument();
  });

  it("root element uses full viewport height with flex column layout", () => {
    render(<AppShell />);
    const root = document.querySelector("div.flex.h-screen.flex-col");
    expect(root).toBeInTheDocument();
  });
});
