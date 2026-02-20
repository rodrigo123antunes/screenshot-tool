import { ThemeProvider, ThemeToggle } from "@/features/theme";

function App() {
  return (
    <ThemeProvider defaultTheme="system">
      <main className="flex min-h-screen items-center justify-center bg-background text-foreground">
        <div className="flex flex-col items-center gap-4">
          <h1 className="text-2xl font-bold">Screenshot Tool</h1>
          <ThemeToggle />
        </div>
      </main>
    </ThemeProvider>
  );
}

export default App;
