import { ThemeToggle } from "@/features/theme";

export function Titlebar() {
  return (
    <header className="flex h-10 shrink-0 items-center border-b border-border bg-background px-3">
      <span className="select-none text-sm font-semibold text-foreground">
        Screenshot Tool
      </span>

      <div className="ml-auto flex items-center gap-1">
        <ThemeToggle />
      </div>
    </header>
  );
}
