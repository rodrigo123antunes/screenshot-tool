import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { LazyStore } from "@tauri-apps/plugin-store";
import {
  ThemeContext,
  type ThemeContextValue,
  type ResolvedTheme,
  type Theme,
} from "./ThemeContext";

const VALID_THEMES: Theme[] = ["light", "dark", "system"];

function isValidTheme(value: unknown): value is Theme {
  return typeof value === "string" && VALID_THEMES.includes(value as Theme);
}

function getSystemTheme(): ResolvedTheme {
  if (typeof window === "undefined" || !window.matchMedia) {
    return "dark";
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function resolveTheme(theme: Theme): ResolvedTheme {
  return theme === "system" ? getSystemTheme() : theme;
}

interface ThemeProviderProps {
  children: React.ReactNode;
  defaultTheme?: Theme;
  storageKey?: string;
}

export function ThemeProvider({
  children,
  defaultTheme = "system",
  storageKey = "theme",
}: ThemeProviderProps) {
  const [theme, setThemeState] = useState<Theme>(defaultTheme);
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(
    resolveTheme(defaultTheme),
  );
  const storeRef = useRef<LazyStore | null>(null);

  // Initialize store ref
  if (storeRef.current === null) {
    storeRef.current = new LazyStore("settings.json");
  }

  // Load persisted theme from Tauri Store on mount
  useEffect(() => {
    const store = storeRef.current;
    if (!store) return;

    store
      .get<string>(storageKey)
      .then((value) => {
        if (isValidTheme(value)) {
          setThemeState(value);
        }
      })
      .catch((err) => {
        console.warn("Failed to load theme from store:", err);
      });
  }, [storageKey]);

  // Apply dark class and update resolvedTheme whenever theme changes
  useEffect(() => {
    const resolved = resolveTheme(theme);
    setResolvedTheme(resolved);

    const root = document.documentElement;
    if (resolved === "dark") {
      root.classList.add("dark");
    } else {
      root.classList.remove("dark");
    }
  }, [theme]);

  // Persist theme to Tauri Store when it changes (skip on initial mount with defaultTheme)
  const isInitialMount = useRef(true);
  useEffect(() => {
    if (isInitialMount.current) {
      isInitialMount.current = false;
      return;
    }

    const store = storeRef.current;
    if (!store) return;

    store
      .set(storageKey, theme)
      .then(() => store.save())
      .catch((err) => {
        console.warn("Failed to persist theme to store:", err);
      });
  }, [theme, storageKey]);

  // Listen for OS theme preference changes
  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = () => {
      if (theme === "system") {
        const resolved = getSystemTheme();
        setResolvedTheme(resolved);

        const root = document.documentElement;
        if (resolved === "dark") {
          root.classList.add("dark");
        } else {
          root.classList.remove("dark");
        }
      }
    };

    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, [theme]);

  const setTheme = useCallback((newTheme: Theme) => {
    setThemeState(newTheme);
  }, []);

  const value = useMemo<ThemeContextValue>(
    () => ({
      theme,
      resolvedTheme,
      setTheme,
    }),
    [theme, resolvedTheme, setTheme],
  );

  return (
    <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
  );
}
