import { render, screen, fireEvent, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock @tauri-apps/plugin-store
const { mockGet, mockSet, mockSave } = vi.hoisted(() => ({
  mockGet: vi.fn(),
  mockSet: vi.fn(),
  mockSave: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-store", () => ({
  LazyStore: class {
    get = mockGet;
    set = mockSet;
    save = mockSave;
  },
}));

import App from "@/App";

describe("App (Integration)", () => {
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

    mockGet.mockResolvedValue(null);
    mockSet.mockResolvedValue(undefined);
    mockSave.mockResolvedValue(undefined);

    document.documentElement.classList.remove("dark");
  });

  it("renders the full app shell without crashing", () => {
    render(<App />);

    // Titlebar is present with app title
    expect(screen.getByText("Screenshot Tool")).toBeInTheDocument();
  });

  it("mounts ThemeProvider + AppShell + Titlebar + ContentArea", () => {
    render(<App />);

    // AppShell root container
    const shell = document.querySelector("div.flex.h-screen.flex-col");
    expect(shell).toBeInTheDocument();

    // Titlebar (header element)
    const header = document.querySelector("header");
    expect(header).toBeInTheDocument();

    // ContentArea (main element)
    const main = document.querySelector("main");
    expect(main).toBeInTheDocument();
  });

  it("renders ThemeToggle in the titlebar", () => {
    render(<App />);

    const button = screen.getByRole("button", { name: /mode|theme/i });
    expect(button).toBeInTheDocument();
  });

  it("applies theme correctly via ThemeProvider", async () => {
    render(<App />);

    // With matchMedia returning dark, system theme should resolve to dark
    await vi.waitFor(() => {
      expect(document.documentElement.classList.contains("dark")).toBe(true);
    });
  });

  it("theme toggle changes theme across the app", async () => {
    // Start with light theme persisted
    mockGet.mockResolvedValue("light");

    await act(async () => {
      render(<App />);
    });

    await vi.waitFor(() => {
      expect(document.documentElement.classList.contains("dark")).toBe(false);
    });

    // Click toggle: light -> dark
    const button = screen.getByRole("button", { name: /mode|theme/i });
    await act(async () => {
      fireEvent.click(button);
    });

    await vi.waitFor(() => {
      expect(document.documentElement.classList.contains("dark")).toBe(true);
    });
  });

  it("loads persisted theme from store on mount", async () => {
    mockGet.mockResolvedValue("dark");

    await act(async () => {
      render(<App />);
    });

    await vi.waitFor(() => {
      expect(mockGet).toHaveBeenCalledWith("theme");
    });

    await vi.waitFor(() => {
      expect(document.documentElement.classList.contains("dark")).toBe(true);
    });
  });
});
