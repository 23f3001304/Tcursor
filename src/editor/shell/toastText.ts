const MAX_TOAST_CHARS = 120;

/** Caps how much of a message the Toast pill (Toast.tsx) renders inline - it's a small, wrapping
 *  pill meant for "Undid"/"Redid", now also carrying Rust's raw `export-warning` strings (Task
 *  11), which can run long (they splice in a raw decode-error message). Truncates on a word
 *  boundary where possible so it doesn't cut mid-word; the source string itself is never altered -
 *  this only shapes what one transient pill shows. */
export function truncateToastText(text: string, max = MAX_TOAST_CHARS): string {
  if (text.length <= max) return text;
  const cut = text.slice(0, max - 1);
  const lastSpace = cut.lastIndexOf(" ");
  return `${lastSpace > max * 0.6 ? cut.slice(0, lastSpace) : cut}…`;
}
