import { ThemeProvider } from "@/features/theme";
import { AppShell } from "@/features/layout";

function App() {
  return (
    <ThemeProvider defaultTheme="system">
      <AppShell />
    </ThemeProvider>
  );
}

export default App;
