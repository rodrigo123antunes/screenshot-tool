import { Titlebar } from "./Titlebar";
import { ContentArea } from "./ContentArea";

interface AppShellProps {
  children?: React.ReactNode;
}

export function AppShell({ children }: AppShellProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background">
      <Titlebar />
      <ContentArea>{children}</ContentArea>
    </div>
  );
}
