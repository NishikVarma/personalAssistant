import { useEffect, useState } from "react";
import { Monitor, Moon, Sun } from "lucide-react";
import { cn } from "@/lib/utils";
import { getStoredTheme, setTheme, type Theme } from "@/lib/theme";

const OPTIONS: { value: Theme; icon: typeof Sun; label: string }[] = [
  { value: "light", icon: Sun, label: "Light theme" },
  { value: "dark", icon: Moon, label: "Dark theme" },
  { value: "system", icon: Monitor, label: "Follow system theme" },
];

export default function ThemeToggle() {
  const [theme, setLocal] = useState<Theme>(() => getStoredTheme());

  useEffect(() => {
    setTheme(theme);
  }, [theme]);

  return (
    <div
      className="flex items-center gap-0.5 rounded-lg border border-border p-0.5"
      role="group"
      aria-label="Color theme"
    >
      {OPTIONS.map(({ value, icon: Icon, label }) => (
        <button
          key={value}
          type="button"
          aria-label={label}
          title={label}
          aria-pressed={theme === value}
          onClick={() => setLocal(value)}
          className={cn(
            "rounded-md p-1.5 text-muted-foreground transition-colors hover:text-foreground",
            theme === value && "bg-accent text-accent-foreground",
          )}
        >
          <Icon className="h-3.5 w-3.5" />
        </button>
      ))}
    </div>
  );
}
