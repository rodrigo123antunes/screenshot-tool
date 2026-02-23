import { ThemeProvider } from "@/features/theme";
import { AppShell } from "@/features/layout";
import { OverlayView } from "@/features/overlay";

function App() {
  const isOverlay = window.location.pathname === "/overlay";

  if (isOverlay) {
    return <OverlayView />;
  }

  return (
    <ThemeProvider defaultTheme="system">
      <AppShell />
    </ThemeProvider>
  );
}

export default App;
