import { useEffect, useRef } from "react";
import { getSettings } from "../../shared/ipc";
import { applyTheme } from "../preferences/applyTheme";
import type { InterfaceSettings, ThemeMode } from "../settings/settings";

export function useHudTheme() {
  const themeRef = useRef<{ theme: ThemeMode; accent: [number, number, number] }>({
    theme: "light",
    accent: [239, 68, 68],
  });

  useEffect(() => {
    getSettings().then((s) => {
      themeRef.current = { theme: s.ui.theme, accent: s.ui.accent };
      applyTheme(s.ui.theme, s.ui.accent);
    });
    const mq = matchMedia("(prefers-color-scheme: dark)");
    const onMqChange = () => applyTheme(themeRef.current.theme, themeRef.current.accent);
    mq.addEventListener("change", onMqChange);
    return () => mq.removeEventListener("change", onMqChange);
  }, []);

  return (ui: InterfaceSettings) => {
    themeRef.current = { theme: ui.theme, accent: ui.accent };
    applyTheme(ui.theme, ui.accent);
  };
}
