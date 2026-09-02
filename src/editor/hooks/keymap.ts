/** The minimal shape `useEditorKeymap` reads off a `KeyboardEvent` - kept separate from the DOM
 *  type so `keyAction` is trivially unit-testable with plain object literals. */
export interface KeyLike { key: string; ctrlKey: boolean; metaKey: boolean; altKey: boolean; repeat: boolean }

export type KeyAction = "delete" | "zoom" | "spotlight" | "play" | "overlay" | null;

/** Minimal shape `useEditorKeymap` needs off `document.activeElement` - kept separate from the
 *  DOM type for the same reason as `KeyLike`: trivially unit-testable with plain objects. */
export interface TargetLike { tagName: string; role: string | null; isContentEditable: boolean }

const TYPING_TAGS = new Set(["INPUT", "TEXTAREA"]);
// Anything that owns its own Space activation and must never have it hijacked by a global
// `preventDefault` - a native `<button>` (Chromium's own click-on-keyup simulation gets skipped
// entirely once `preventDefault` runs on the keydown) or a custom widget that manages Space
// itself (`Picker`'s menu-open/select - M4). Per WAI-ARIA authoring practices, `role="slider"`
// does NOT list Space as an activation key (only arrow keys move it) - `Slider.tsx` doesn't
// handle it either, so including "slider" here made Space a dead key when one had focus (neither
// the slider nor the global "toggle play" fired); review round 1 minor - removed.
const SPACE_OWNING_TAGS = new Set(["BUTTON", "SELECT"]);
const SPACE_OWNING_ROLES = new Set(["switch", "button", "checkbox", "radio", "menuitem", "tab", "combobox", "option"]);

/** True while typing - the ORIGINAL guard, applies to every shortcut equally: typing "s" or "z"
 *  into a text field must never add a spotlight/zoom. */
export function isTypingTarget(t: TargetLike): boolean {
  return t.isContentEditable || TYPING_TAGS.has(t.tagName);
}

/** True when `t` handles Space itself - the global handler must leave it alone rather than
 *  `preventDefault` it into "toggle play" (M4 scenarios A/B: a focused Switch/Picker never got
 *  its own Space activation because the global listener ate the keydown first). */
export function ownsSpace(t: TargetLike): boolean {
  return SPACE_OWNING_TAGS.has(t.tagName) || (t.role !== null && SPACE_OWNING_ROLES.has(t.role));
}

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

/** The FULL decision `useEditorKeymap` needs for one keydown, folding in the DOM/modal context
 *  `keyAction` alone can't see - kept separate (rather than widening `keyAction`'s own signature)
 *  so `keyAction`'s existing key-mapping tests stay untouched. Every shortcut is inert while a
 *  modal (Export/Settings/Shortcuts/a Confirm dialog/the AI director's own scrim) sits on top -
 *  a `z` typed behind the Export dialog must not silently add a zoom (M4 scenario C). Typing in a
 *  field is still checked first and blocks every action, exactly like before; Space additionally
 *  backs off for any control that owns its own Space activation, WITHOUT blocking z/s/delete/
 *  overlay for that same target - Tab-focusing a button and pressing "z" is unaffected.
 *
 *  `?` is special-cased AHEAD of the modal bail (review round 1 minor): it's the toggle that
 *  OPENS `ShortcutsOverlay` in the first place, and `useEditorKeymap`'s `onOverlay` wiring is a
 *  toggle, not a one-way open - so once the overlay is the CURRENT modal (`ctx.shortcutsOpen`),
 *  pressing `?` again must still be able to close it, exactly like Escape/a scrim click already
 *  can. It stays blocked behind any OTHER modal (`ctx.modalOpen && !ctx.shortcutsOpen`), same as
 *  every other shortcut - `?` must not pop the overlay open ON TOP of, say, the Export dialog. */
export function resolveKeyAction(e: KeyLike, ctx: { hasSel: boolean; modalOpen: boolean; shortcutsOpen: boolean; target: TargetLike }): KeyAction {
  if (isTypingTarget(ctx.target)) return null;
  if (e.key === "?") return (ctx.modalOpen && !ctx.shortcutsOpen) ? null : keyAction(e, ctx.hasSel);
  if (ctx.modalOpen) return null;
  const action = keyAction(e, ctx.hasSel);
  if (action === "play" && ownsSpace(ctx.target)) return null;
  return action;
}
