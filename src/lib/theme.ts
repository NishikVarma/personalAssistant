export type Theme = "light" | "dark" | "system";

const STORAGE_KEY = "theme";

function prefersDark(): boolean {
  return (
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
  );
}

export function getStoredTheme(): Theme {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "light" || stored === "dark") return stored;
  } catch {
    // storage unavailable — fall through to system default
  }
  return "system";
}

export function applyTheme(theme: Theme): void {
  if (typeof document === "undefined") return;
  const dark = theme === "dark" || (theme === "system" && prefersDark());
  document.documentElement.classList.toggle("dark", dark);
}

export function setTheme(theme: Theme): void {
  try {
    localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    // ignore persistence failures; the choice still applies for this session
  }
  applyTheme(theme);
}

function onSystemChange(): void {
  if (getStoredTheme() === "system") applyTheme("system");
}

let listening = false;

/** Applies the stored theme and keeps system mode in sync with OS changes. */
export function initTheme(): void {
  applyTheme(getStoredTheme());
  if (listening || typeof window.matchMedia !== "function") return;
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  const add = (cb: () => void) =>
    typeof media.addEventListener === "function"
      ? media.addEventListener("change", cb)
      : media.addListener(cb);
  add(onSystemChange);
  listening = true;
}
