interface ContentAreaProps {
  children?: React.ReactNode;
}

export function ContentArea({ children }: ContentAreaProps) {
  return (
    <main className="flex-1 overflow-auto bg-background text-foreground">
      {children}
    </main>
  );
}
