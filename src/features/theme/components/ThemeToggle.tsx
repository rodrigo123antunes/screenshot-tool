import { Moon, Sun, Monitor } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTheme } from "../hooks/useTheme";
import type { Theme } from "../context/ThemeContext";

const THEME_CYCLE: Theme[] = ["light", "dark", "system"];

const THEME_ICONS = {
  light: Sun,
  dark: Moon,
  system: Monitor,
} as const;

const THEME_LABELS = {
  light: "Light mode",
  dark: "Dark mode",
  system: "System theme",
} as const;

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();

  const handleToggle = () => {
    const currentIndex = THEME_CYCLE.indexOf(theme);
    const nextIndex = (currentIndex + 1) % THEME_CYCLE.length;
    setTheme(THEME_CYCLE[nextIndex]);
  };

  const Icon = THEME_ICONS[theme];

  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={handleToggle}
      aria-label={THEME_LABELS[theme]}
    >
      <Icon className="size-4" />
    </Button>
  );
}
