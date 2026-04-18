import { createSignal } from "solid-js";

export type Theme = "system" | "light" | "dark";

const STORAGE_KEY = "clipbrain-theme";

function getStoredTheme(): Theme {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "light" || stored === "dark" || stored === "system") return stored;
  } catch {}
  return "system";
}

function getResolvedTheme(theme: Theme): "light" | "dark" {
  if (theme !== "system") return theme;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

const [theme, setThemeInternal] = createSignal<Theme>(getStoredTheme());
const [resolvedTheme, setResolvedTheme] = createSignal<"light" | "dark">(
  getResolvedTheme(getStoredTheme())
);

function applyTheme(t: Theme) {
  const resolved = getResolvedTheme(t);
  setResolvedTheme(resolved);
  document.documentElement.setAttribute("data-theme", resolved);
}

export function setTheme(t: Theme) {
  setThemeInternal(t);
  try {
    localStorage.setItem(STORAGE_KEY, t);
  } catch {}
  applyTheme(t);
}

// Initialize on load
applyTheme(getStoredTheme());

// Listen for system theme changes
if (typeof window !== "undefined") {
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if (theme() === "system") {
      applyTheme("system");
    }
  });
}

export { theme, resolvedTheme };
