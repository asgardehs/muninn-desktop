/**
 * useTheme — React hook for Nótt/Dagr theme state.
 *
 * - Reads initial theme from localStorage, then falls back to
 *   the OS color-scheme preference.
 * - Writes the `data-theme` attribute on <html> so theme.css applies.
 * - Persists changes to localStorage.
 *
 * The hook is the single source of truth; CodeMirror components read
 * `theme` from this hook and pass it to themeByName().
 */

import { useCallback, useEffect, useState } from "react";

export type Theme = "nott" | "dagr";

const STORAGE_KEY = "asgard-theme";

function readInitialTheme(): Theme {
  if (typeof window === "undefined") return "nott";

  const stored = window.localStorage.getItem(STORAGE_KEY);
  if (stored === "nott" || stored === "dagr") return stored;

  return window.matchMedia("(prefers-color-scheme: light)").matches
    ? "dagr"
    : "nott";
}

export function useTheme() {
  const [theme, setThemeState] = useState<Theme>(readInitialTheme);

  // Sync the attribute and localStorage whenever theme changes.
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    try {
      window.localStorage.setItem(STORAGE_KEY, theme);
    } catch {
      // localStorage may be unavailable (private mode, etc.); non-fatal.
    }
  }, [theme]);

  const setTheme = useCallback((next: Theme) => setThemeState(next), []);

  const toggle = useCallback(
    () => setThemeState((prev) => (prev === "nott" ? "dagr" : "nott")),
    [],
  );

  return { theme, setTheme, toggle };
}
