import type { Tab } from "./PanelTabs";

const KEY = "tcursor.editor.panel";

export function nextTab(current: Tab | null, clicked: Tab): Tab | null {
  return current === clicked ? null : clicked;
}

export function readPanelTab(valid: readonly Tab[]): Tab | null {
  try {
    const v = localStorage.getItem(KEY);
    if (v === "") return null;
    return v !== null && (valid as readonly string[]).includes(v) ? (v as Tab) : "ai";
  } catch {
    return "ai";
  }
}

export function writePanelTab(tab: Tab | null): void {
  try {
    localStorage.setItem(KEY, tab ?? "");
  } catch {}
}
