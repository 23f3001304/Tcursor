# src/editor/controls/ConfirmDialog.tsx

A small modal confirm dialog: a scrim plus a centered card with a title, body, an optional "don't ask again" checkbox, and Cancel / confirm buttons. Reusable across the editor.

## ConfirmDialog

```tsx
export function ConfirmDialog({ open, title, body, confirmLabel, danger, dontAskLabel, onConfirm, onCancel }: {
  open: boolean; title: string; body: string; confirmLabel: string; danger?: boolean;
  dontAskLabel?: string; onConfirm: (dontAsk: boolean) => void; onCancel: () => void;
}): JSX.Element
```

Animated with Motion (`AnimatePresence` - scrim fade + card scale). The scrim click and Cancel both call `onCancel`; the confirm button calls `onConfirm(dontAsk)` with the checkbox state so the caller decides whether to persist the suppression. `danger` styles the confirm button red (`--e-danger`/`--e-dangerh`) for destructive actions. When `dontAskLabel` is omitted the checkbox is hidden. The checkbox's tick is drawn at `--e-on-fill` - the palette's "label on a saturated fill" token, white in both themes - because the box behind it is `--e-primary` once checked, not a surface. The confirm button is a `motion.button` carrying the app-wide press spring (design/premium-pass D6, `whileTap: { scale: 0.96 }`); Cancel stays a plain button, unmotioned, matching the "hero gets the spring, quiet chrome doesn't" split used elsewhere (`Transport`'s Play, `ExportDialog`/`ExportProgress`'s primary buttons).
