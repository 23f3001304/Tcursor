import type { Tab } from "./panelTabs";

// Which panel the rail is showing, or none at all. `null` is a real, reachable state since the
// panel slot became collapsible: the rail stays, the 320px column goes, and the stage takes the
// width. The two rules live here (rather than inline in the shell) so they can be tested without
// mounting an editor, and so the rail and the panels cannot disagree about what a click means.

const KEY = "tcursor.editor.panel";

/** What clicking rail tab `clicked` should leave showing. Clicking the tab that is already open
 *  closes the panel; clicking any other tab opens that one - including from closed, which is what
 *  makes the rail a toggle rather than a one-way door. */
export function nextTab(current: Tab | null, clicked: Tab): Tab | null {
  return current === clicked ? null : clicked;
}

/** The panel the editor last had open, or `null` for "it was collapsed". Storage can throw outright
 *  (a webview with site data blocked) and the editor must still open, so every failure - a throw, a
 *  missing key, a value from a build whose tab no longer exists - reads as the default `"ai"` tab,
 *  never as a collapsed editor the user did not ask for. The empty string is the ONE value that
 *  means collapsed, which is why it is stored rather than removing the key. */
export function readPanelTab(valid: readonly Tab[]): Tab | null {
  try {
    const v = localStorage.getItem(KEY);
    if (v === "") return null;
    return v !== null && (valid as readonly string[]).includes(v) ? (v as Tab) : "ai";
  } catch { return "ai"; }
}

/** Remember the open panel (or that there is none). Silent on failure, for the same reason. */
export function writePanelTab(tab: Tab | null): void {
  try { localStorage.setItem(KEY, tab ?? ""); } catch { /* nothing to remember with */ }
}
