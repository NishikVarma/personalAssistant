import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { applyTheme, getStoredTheme, setTheme, type Theme } from "@/lib/theme";

function stubMatchMedia(matchesDark: boolean) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: query.includes("dark") ? matchesDark : false,
      media: query,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
    }),
  });
}

beforeEach(() => {
  stubMatchMedia(false);
  localStorage.clear();
  document.documentElement.classList.remove("dark");
});

afterEach(() => {
  localStorage.clear();
  document.documentElement.classList.remove("dark");
});

describe("theme", () => {
  it("defaults to system and applies light when OS is light", () => {
    expect(getStoredTheme()).toBe("system" satisfies Theme);
    applyTheme("system");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("applies dark in system mode when OS prefers dark", () => {
    stubMatchMedia(true);
    applyTheme("system");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("persists explicit choices and toggles the class", () => {
    setTheme("dark");
    expect(localStorage.getItem("theme")).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    setTheme("light");
    expect(localStorage.getItem("theme")).toBe("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("explicit dark wins over a light system preference", () => {
    stubMatchMedia(false);
    applyTheme("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("survives unavailable storage without throwing", () => {
    const original = Storage.prototype.setItem;
    Storage.prototype.setItem = () => {
      throw new Error("quota");
    };
    expect(() => setTheme("dark")).not.toThrow();
    Storage.prototype.setItem = original;
  });
});
