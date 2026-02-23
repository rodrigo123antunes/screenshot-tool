import { render, screen, fireEvent, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

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

// Mock @tauri-apps/api/event (needed by OverlayView via useCaptureEvents)
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));

// Mock @tauri-apps/api/core (needed by OverlayView)
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path: string) => `asset://localhost${path}`),
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

describe("App routing", () => {
  const originalLocation = window.location;

  afterEach(() => {
    // Restore original location after each test
    Object.defineProperty(window, "location", {
      writable: true,
      value: originalLocation,
    });
  });

  it("renderiza OverlayView diretamente quando pathname é /overlay (sem ThemeProvider ou AppShell)", () => {
    Object.defineProperty(window, "location", {
      writable: true,
      value: { pathname: "/overlay" },
    });

    render(<App />);

    // OverlayView deve estar presente
    expect(screen.getByTestId("overlay-view")).toBeInTheDocument();

    // AppShell NÃO deve estar presente (sem header/main do AppShell)
    expect(document.querySelector("header")).not.toBeInTheDocument();
    expect(document.querySelector("main")).not.toBeInTheDocument();

    // Título da aplicação (Titlebar do AppShell) NÃO deve estar presente
    expect(screen.queryByText("Screenshot Tool")).not.toBeInTheDocument();
  });

  it("não renderiza ThemeProvider nem AppShell quando pathname é /overlay", () => {
    Object.defineProperty(window, "location", {
      writable: true,
      value: { pathname: "/overlay" },
    });

    render(<App />);

    // OverlayView está presente
    expect(screen.getByTestId("overlay-view")).toBeInTheDocument();

    // ThemeToggle (do Titlebar dentro do AppShell) NÃO deve estar presente
    expect(screen.queryByRole("button", { name: /mode|theme/i })).not.toBeInTheDocument();
  });

  it("renderiza ThemeProvider + AppShell quando pathname é / (path padrão)", () => {
    Object.defineProperty(window, "location", {
      writable: true,
      value: { pathname: "/" },
    });

    render(<App />);

    // AppShell deve estar presente
    expect(document.querySelector("header")).toBeInTheDocument();
    expect(document.querySelector("main")).toBeInTheDocument();

    // OverlayView NÃO deve estar presente
    expect(screen.queryByTestId("overlay-view")).not.toBeInTheDocument();
  });

  it("renderiza ThemeProvider + AppShell quando pathname não é /overlay", () => {
    Object.defineProperty(window, "location", {
      writable: true,
      value: { pathname: "/some-other-path" },
    });

    render(<App />);

    // AppShell deve estar presente
    expect(document.querySelector("header")).toBeInTheDocument();

    // OverlayView NÃO deve estar presente
    expect(screen.queryByTestId("overlay-view")).not.toBeInTheDocument();
  });
});
