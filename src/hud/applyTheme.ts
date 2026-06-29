import type { ThemeMode } from "./settings";

/** Apply the app theme + accent to the document root. In "system" mode the
 *  light/dark choice follows the OS via prefers-color-scheme. */
export function applyTheme(theme: ThemeMode, accent: [number, number, number]) {
  const dark = theme === "dark" || (theme === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  document.documentElement.style.setProperty("--accent", `rgb(${accent[0]}, ${accent[1]}, ${accent[2]})`);
}
