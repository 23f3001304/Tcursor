import type { ThemeMode } from "../settings/settings";

export function applyTheme(theme: ThemeMode, accent: [number, number, number]) {
  const systemDark = typeof matchMedia === "function" && matchMedia("(prefers-color-scheme: dark)").matches;
  const dark = theme === "dark" || (theme === "system" && systemDark);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  document.documentElement.style.setProperty("--accent", `rgb(${accent[0]}, ${accent[1]}, ${accent[2]})`);
}
