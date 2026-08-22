import type { LucideIcon } from "lucide-react";
import { Hammer } from "lucide-react";

interface PlaceholderProps {
  title: string;
  phase: number;
  icon: LucideIcon;
}

export default function Placeholder({ title, phase, icon: Icon }: PlaceholderProps) {
  return (
    <section>
      <header className="mb-8">
        <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
        <p className="mt-1 text-sm text-muted-foreground">Arrives in Phase {phase}</p>
      </header>
      <div className="flex min-h-[50vh] items-center justify-center rounded-xl border border-dashed border-border">
        <div className="flex flex-col items-center gap-3 text-center text-muted-foreground">
          <Hammer className="h-6 w-6" />
          <Icon className="h-10 w-10 opacity-40" />
          <div>
            <p className="font-medium text-foreground">Nothing here yet</p>
            <p className="mt-1 max-w-sm text-sm">
              This section is built in a later vertical slice. The database schema for it already
              exists.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
