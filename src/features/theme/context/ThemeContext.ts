import { createContext } from "react";

export type Theme = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

export interface ThemeContextValue {
  /** Preferência do usuário: 'light', 'dark', ou 'system' */
  theme: Theme;
  /** Tema efetivamente aplicado (resolvido de 'system' → 'light' ou 'dark') */
  resolvedTheme: ResolvedTheme;
  /** Altera a preferência de tema e persiste no Tauri Store */
  setTheme: (theme: Theme) => void;
}

export const ThemeContext = createContext<ThemeContextValue | undefined>(
  undefined,
);
