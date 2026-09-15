export interface KeyLike {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  repeat: boolean;
  shiftKey?: boolean;
}

export type KeyAction = "delete" | "zoom" | "spotlight" | "play" | "overlay" | "deselect" | null;

export interface TargetLike {
  tagName: string;
  role: string | null;
  isContentEditable: boolean;
}

const TYPING_TAGS = new Set(["INPUT", "TEXTAREA"]);

const SPACE_OWNING_TAGS = new Set(["BUTTON", "SELECT"]);
const SPACE_OWNING_ROLES = new Set([
  "switch",
  "button",
  "checkbox",
  "radio",
  "menuitem",
  "tab",
  "combobox",
  "option",
]);

export function isTypingTarget(t: TargetLike): boolean {
  return t.isContentEditable || TYPING_TAGS.has(t.tagName);
}

export function ownsSpace(t: TargetLike): boolean {
  return SPACE_OWNING_TAGS.has(t.tagName) || (t.role !== null && SPACE_OWNING_ROLES.has(t.role));
}

export function keyAction(e: KeyLike, hasSel: boolean): KeyAction {
  if (e.ctrlKey || e.metaKey || e.altKey) return null;
  if (e.key === "?") return e.repeat ? null : "overlay";
  const k = e.key.toLowerCase();
  if (k === "z") return e.repeat ? null : "zoom";
  if (k === "s") return e.repeat ? null : "spotlight";
  if (k === "delete" || k === "backspace") return hasSel ? "delete" : null;
  if (k === "escape") return hasSel ? "deselect" : null;
  if (k === " ") return "play";
  return null;
}

export function resolveKeyAction(
  e: KeyLike,
  ctx: { hasSel: boolean; modalOpen: boolean; shortcutsOpen: boolean; target: TargetLike },
): KeyAction {
  if (isTypingTarget(ctx.target)) return null;
  if (e.key === "?") return ctx.modalOpen && !ctx.shortcutsOpen ? null : keyAction(e, ctx.hasSel);
  if (ctx.modalOpen) return null;
  const action = keyAction(e, ctx.hasSel);
  if (action === "play" && ownsSpace(ctx.target)) return null;
  return action;
}
