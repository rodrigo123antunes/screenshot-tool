import { render, screen, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { ThemeProvider } from "../context/ThemeProvider";
import { useTheme } from "../hooks/useTheme";

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

function ThemeConsumer() {
  const { theme, resolvedTheme, setTheme } = useTheme();
  return (
    <div>
      <span data-testid="theme">{theme}</span>
      <span data-testid="resolved">{resolvedTheme}</span>
      <button onClick={() => setTheme("dark")}>Set Dark</button>
      <button onClick={() => setTheme("light")}>Set Light</button>
      <button onClick={() => setTheme("system")}>Set System</button>
    </div>
  );
}

describe("ThemeProvider", () => {
  let matchMediaMock: ReturnType<typeof vi.fn>;
  let listeners: Array<(e: { matches: boolean }) => void>;

  beforeEach(() => {
    listeners = [];
    matchMediaMock = vi.fn().mockImplementation((query: string) => ({
      matches: query === "(prefers-color-scheme: dark)" ? true : false,
      media: query,
      addEventListener: (_event: string, cb: (e: { matches: boolean }) => void) => {
        listeners.push(cb);
      },
      removeEventListener: (_event: string, cb: (e: { matches: boolean }) => void) => {
        listeners = listeners.filter((l) => l !== cb);
      },
    }));
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: matchMediaMock,
    });

    mockGet.mockResolvedValue(null);
    mockSet.mockResolvedValue(undefined);
    mockSave.mockResolvedValue(undefined);

    document.documentElement.classList.remove("dark");
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("initializes with defaultTheme 'system' and resolves based on matchMedia", () => {
    render(
      <ThemeProvider defaultTheme="system">
        <ThemeConsumer />
      </ThemeProvider>,
    );

    expect(screen.getByTestId("theme")).toHaveTextContent("system");
    expect(screen.getByTestId("resolved")).toHaveTextContent("dark");
  });

  it("applies 'dark' class on documentElement when resolvedTheme is 'dark'", async () => {
    render(
      <ThemeProvider defaultTheme="dark">
        <ThemeConsumer />
      </ThemeProvider>,
    );

    await vi.waitFor(() => {
      expect(document.documentElement.classList.contains("dark")).toBe(true);
    });
  });

  it("removes 'dark' class when resolvedTheme is 'light'", async () => {
    document.documentElement.classList.add("dark");

    render(
      <ThemeProvider defaultTheme="light">
        <ThemeConsumer />
      </ThemeProvider>,
    );

    await vi.waitFor(() => {
      expect(document.documentElement.classList.contains("dark")).toBe(false);
    });
  });

  it("loads theme from Tauri Store on mount", async () => {
    mockGet.mockResolvedValue("dark");

    await act(async () => {
      render(
        <ThemeProvider defaultTheme="system">
          <ThemeConsumer />
        </ThemeProvider>,
      );
    });

    await vi.waitFor(() => {
      expect(screen.getByTestId("theme")).toHaveTextContent("dark");
    });
    expect(mockGet).toHaveBeenCalledWith("theme");
  });

  it("falls back to 'system' when Store read fails", async () => {
    mockGet.mockRejectedValue(new Error("Store error"));
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

    render(
      <ThemeProvider defaultTheme="system">
        <ThemeConsumer />
      </ThemeProvider>,
    );

    await vi.waitFor(() => {
      expect(warnSpy).toHaveBeenCalled();
    });
    expect(screen.getByTestId("theme")).toHaveTextContent("system");

    warnSpy.mockRestore();
  });

  it("falls back to 'system' when Store returns invalid value", async () => {
    mockGet.mockResolvedValue("invalid-theme");

    render(
      <ThemeProvider defaultTheme="system">
        <ThemeConsumer />
      </ThemeProvider>,
    );

    // Wait for async store.get to resolve
    await vi.waitFor(() => {
      expect(mockGet).toHaveBeenCalled();
    });
    expect(screen.getByTestId("theme")).toHaveTextContent("system");
  });

  it("persists theme to Tauri Store on setTheme", async () => {
    render(
      <ThemeProvider defaultTheme="system">
        <ThemeConsumer />
      </ThemeProvider>,
    );

    await act(async () => {
      screen.getByText("Set Dark").click();
    });

    await vi.waitFor(() => {
      expect(mockSet).toHaveBeenCalledWith("theme", "dark");
    });
    expect(mockSave).toHaveBeenCalled();
  });

  it("updates resolvedTheme when OS preference changes and theme is 'system'", async () => {
    render(
      <ThemeProvider defaultTheme="system">
        <ThemeConsumer />
      </ThemeProvider>,
    );

    // Initially dark (matchMedia returns true)
    expect(screen.getByTestId("resolved")).toHaveTextContent("dark");

    // Simulate OS switching to light
    matchMediaMock.mockImplementation((query: string) => ({
      matches: query === "(prefers-color-scheme: dark)" ? false : false,
      media: query,
      addEventListener: (_event: string, cb: (e: { matches: boolean }) => void) => {
        listeners.push(cb);
      },
      removeEventListener: vi.fn(),
    }));

    await act(async () => {
      for (const listener of listeners) {
        listener({ matches: false });
      }
    });

    await vi.waitFor(() => {
      expect(screen.getByTestId("resolved")).toHaveTextContent("light");
    });
  });

  it("does not update resolvedTheme on OS change when theme is not 'system'", async () => {
    render(
      <ThemeProvider defaultTheme="dark">
        <ThemeConsumer />
      </ThemeProvider>,
    );

    expect(screen.getByTestId("resolved")).toHaveTextContent("dark");

    // Simulate OS switching to light - should NOT affect since theme is 'dark' not 'system'
    await act(async () => {
      for (const listener of listeners) {
        listener({ matches: false });
      }
    });

    expect(screen.getByTestId("resolved")).toHaveTextContent("dark");
  });
});
