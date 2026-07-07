# src/editor/controls/ConfirmDialog.tsx

A small modal confirm dialog: a scrim plus a centered card with a title, body, an optional "don't ask again" checkbox, and Cancel / confirm buttons. Reusable across the editor.

## ConfirmDialog

```tsx
export function ConfirmDialog({ open, title, body, confirmLabel, danger, dontAskLabel, onConfirm, onCancel }: {
  open: boolean; title: string; body: string; confirmLabel: string; danger?: boolean;
  dontAskLabel?: string; onConfirm: (dontAsk: boolean) => void; onCancel: () => void;
}): JSX.Element
```

Animated with Motion (`AnimatePresence` - scrim fade + card scale). The scrim click and Cancel both call `onCancel`; the confirm button calls `onConfirm(dontAsk)` with the checkbox state so the caller decides whether to persist the suppression. `danger` styles the confirm button red for destructive actions. When `dontAskLabel` is omitted the checkbox is hidden.
