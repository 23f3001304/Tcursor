# src/editor/inspectors/CaptionTextField.tsx

The caption's own words, edited in place.

## CaptionTextField

```tsx
export function CaptionTextField({ value, onCommit, ariaLabel }: {
  value: string; onCommit: (text: string) => void; ariaLabel?: string;
}): JSX.Element
```

A DRAFT field, not a live-edit one - the one inspector control in the editor that is. Every other inspector control applies its op on change because a slider's intermediate values are all valid and `useEditHistory` coalesces a drag into one undo step; a keystroke is different. Applying per keystroke would put a hundred undo steps behind one sentence, write `edit.json` a hundred times, and re-render the preview on each. So the textarea owns the text while it is focused and commits ONCE:

- **blur** commits.
- **Enter** blurs, which commits. It is not a newline: a caption wraps by itself at the renderer's own character budget (`captionlayout::wrap_lines`), so a hand-typed break would be discarded by the export and the preview both, and a control whose effect is thrown away is worse than no control. Shift+Enter is not special-cased for the same reason.
- **Escape** puts the caption's text back and gives up focus - the only exit that changes nothing.
- An **emptied** field reverts instead of committing: a caption with no text is a pill on the timeline that paints nothing, and Delete already exists for meaning it.
- **Nothing changed** commits nothing, so an accidental focus-and-blur never lands an undo step.

`value` changing UNDER the field (another op, an undo, a different caption selected) replaces the draft - but never while it is focused, which would eat the edit in progress. That is what the `editing` ref guards.

`onKeyDown` stops propagation. Every editor shortcut is a bare key (`keymap.ts`), so a caption containing the letter "s" must not also seek.

### Used by

- `src/editor/inspectors/CaptionInspector.tsx`
