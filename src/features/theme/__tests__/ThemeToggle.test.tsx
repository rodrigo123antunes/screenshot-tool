import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { ThemeToggle } from "../components/ThemeToggle";
import { ThemeProvider } from "../context/ThemeProvider";

vi.mock("@tauri-apps/plugin-store", () => ({
  LazyStore: class {
    get = vi.fn().mockResolvedValue(null);
    set = vi.fn().mockResolvedValue(undefined);
    save = vi.fn().mockResolvedValue(undefined);
  },
}));

function renderWithTheme(defaultTheme: "light" | "dark" | "system" = "light") {
  return render(
    <ThemeProvider defaultTheme={defaultTheme}>
      <ThemeToggle />
    </ThemeProvider>,
  );
}

describe("ThemeToggle", () => {
  beforeEach(() => {
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn().mockImplementation((query: string) => ({
        matches: query === "(prefers-color-scheme: dark)",
        media: query,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      })),
    });
    document.documentElement.classList.remove("dark");
  });

  it("renders a button with accessible label", () => {
    renderWithTheme("light");
    const button = screen.getByRole("button");
    expect(button).toBeInTheDocument();
    expect(button).toHaveAttribute("aria-label");
  });

  it("cycles theme on click: light -> dark -> system -> light", async () => {
    renderWithTheme("light");
    const button = screen.getByRole("button");

    // light -> dark
    expect(button).toHaveAttribute("aria-label", "Light mode");
    fireEvent.click(button);
    await vi.waitFor(() => {
      expect(button).toHaveAttribute("aria-label", "Dark mode");
    });

    // dark -> system
    fireEvent.click(button);
    await vi.waitFor(() => {
      expect(button).toHaveAttribute("aria-label", "System theme");
    });

    // system -> light
    fireEvent.click(button);
    await vi.waitFor(() => {
      expect(button).toHaveAttribute("aria-label", "Light mode");
    });
  });

  it("applies dark class when toggled to dark mode", async () => {
    renderWithTheme("light");
    const button = screen.getByRole("button");

    fireEvent.click(button);

    await vi.waitFor(() => {
      expect(document.documentElement.classList.contains("dark")).toBe(true);
    });
  });
});
