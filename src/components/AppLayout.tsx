import { NavLink, Outlet } from "react-router-dom";
import { Toaster } from "sonner";
import { cn } from "@/lib/utils";
import { SECTIONS } from "@/lib/sections";
import { ipc } from "@/lib/ipc";
import ThemeToggle from "@/components/ThemeToggle";
import { useEffect, useState } from "react";

export default function AppLayout() {
  const [version, setVersion] = useState<string>("");

  useEffect(() => {
    ipc
      .appInfo()
      .then((info) => setVersion(`v${info.appVersion}`))
      .catch(() => setVersion(""));
  }, []);

  return (
    <div className="flex h-screen overflow-hidden">
      <aside className="flex w-60 shrink-0 flex-col border-r border-border bg-sidebar/40">
        <div className="px-5 py-5">
          <p className="text-sm font-semibold leading-tight">Job Application</p>
          <p className="text-sm font-semibold leading-tight text-primary">Copilot</p>
        </div>
        <nav className="flex-1 space-y-1 px-3">
          {SECTIONS.map(({ path, label, icon: Icon }) => (
            <NavLink
              key={path}
              to={path}
              end={path === "/"}
              className={({ isActive }) =>
                cn(
                  "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                  isActive
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground hover:bg-accent/50 hover:text-foreground",
                )
              }
            >
              <Icon className="h-4 w-4" />
              {label}
            </NavLink>
          ))}
        </nav>
        <div className="flex items-center justify-between gap-2 px-4 py-4">
          <span className="text-xs text-muted-foreground">{version}</span>
          <ThemeToggle />
        </div>
      </aside>
      <main className="flex-1 overflow-y-auto p-8">
        <Outlet />
      </main>
      <Toaster richColors position="bottom-right" closeButton />
    </div>
  );
}
