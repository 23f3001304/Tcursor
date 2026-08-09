/** The minimal shape `useEditorKeymap` reads off a `KeyboardEvent` - kept separate from the DOM
 *  type so `keyAction` is trivially unit-testable with plain object literals. */
export interface KeyLike { key: string; ctrlKey: boolean; metaKey: boolean; altKey: boolean; repeat: boolean }

export type KeyAction = "delete" | "zoom" | "spotlight" | "play" | "overlay" | null;

/** Pure decision for one keydown: which editor shortcut (if any) it triggers.
 *
 *  Guarded against ANY modifier (Ctrl/Cmd/Alt) so a browser/OS chord never also fires an editor
 *  shortcut whose bare key happens to collide - `Ctrl+Z` (undo) and the bare `z` (add zoom) is
 *  exactly this collision: the async add-zoom apply usually won the race against the synchronous
 *  undo, so Ctrl+Z APPEARED to undo by adding a zoom instead. Shift is deliberately NOT in the
 *  guard set - `?` is Shift+/ on a US layout, and the browser already delivers the shifted
 *  character in `.key`, so matching `"?"` directly handles it without special-casing `shiftKey`.
 *
 *  Zoom/spotlight/overlay also guard against key-repeat (holding Z/S must not spam regions at OS
 *  repeat rate, and holding `?` must not flicker the ShortcutsOverlay open/closed at the same
 *  rate - `useEditorKeymap` toggles it on every fired "overlay" action); delete/play do NOT -
 *  holding Space to keep playing, or Delete to clear several selections in a row, is fine.
 *  `hasSel` gates delete: nothing to delete without a selection. */
export function keyAction(e: KeyLike, hasSel: boolean): KeyAction {
  if (e.ctrlKey || e.metaKey || e.altKey) return null;
  if (e.key === "?") return e.repeat ? null : "overlay";
  const k = e.key.toLowerCase();
  if (k === "z") return e.repeat ? null : "zoom";
  if (k === "s") return e.repeat ? null : "spotlight";
  if (k === "delete" || k === "backspace") return hasSel ? "delete" : null;
  if (k === " ") return "play";
  return null;
}
