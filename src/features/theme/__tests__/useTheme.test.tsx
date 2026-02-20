import { renderHook, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { useTheme } from "../hooks/useTheme";
import { ThemeProvider } from "../context/ThemeProvider";

vi.mock("@tauri-apps/plugin-store", () => ({
  LazyStore: class {
    get = vi.fn().mockResolvedValue(null);
    set = vi.fn().mockResolvedValue(undefined);
    save = vi.fn().mockResolvedValue(undefined);
  },
}));

describe("useTheme", () => {
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
  });

  it("throws error when used outside ThemeProvider", () => {
    // Suppress console.error from React error boundary
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});

    expect(() => renderHook(() => useTheme())).toThrow(
      "useTheme must be used within a <ThemeProvider>",
    );

    spy.mockRestore();
  });

  it("returns theme, resolvedTheme, and setTheme when used inside ThemeProvider", () => {
    const { result } = renderHook(() => useTheme(), {
      wrapper: ({ children }) => (
        <ThemeProvider defaultTheme="system">{children}</ThemeProvider>
      ),
    });

    expect(result.current.theme).toBe("system");
    expect(result.current.resolvedTheme).toBeDefined();
    expect(typeof result.current.setTheme).toBe("function");
  });

  it("setTheme updates the theme value", async () => {
    const { result } = renderHook(() => useTheme(), {
      wrapper: ({ children }) => (
        <ThemeProvider defaultTheme="system">{children}</ThemeProvider>
      ),
    });

    await act(async () => {
      result.current.setTheme("dark");
    });

    expect(result.current.theme).toBe("dark");
    expect(result.current.resolvedTheme).toBe("dark");
  });
});
