# src/editor/panels/layout/PresetNameField.tsx

The saved-looks section's one text field. Split out of `LayoutPresetList.tsx` because it has two callers now - the list's "save the current look" form and each row's rename - and a private helper cannot serve both once the row is its own file.

## PresetNameField

```tsx
export function PresetNameField({ initial, check, onCommit, onCancel }: {
  initial: string;
  check: (name: string) => string | null;
  onCommit: (name: string) => void;
  onCancel: () => void;
}): JSX.Element
```

Used for **both** "save the current look" (`initial: ""`) and "rename this look" (`initial: p.name`). Enter commits, Esc cancels, and a refused name says why underneath (`.e-lay-err`, `role="alert"`) instead of silently doing nothing.

The rule comes in as `check`, not as a hardcoded call: the caller passes `presetNameError(n, presets)` for a new look and `presetNameError(n, presets, p.id)` for a rename, so both fields enforce the same rule and only the "except myself" clause differs. The error clears on the next keystroke. It autofocuses because it only ever exists in the first place because the user just asked for it, and `maxLength` is `MAX_PRESET_NAME + 1` deliberately - one character past the limit, so a user who overshoots gets the sentence explaining the limit rather than a field that silently stops accepting letters.

### Used by

- `src/editor/panels/layout/LayoutPresetList.tsx` - the new-look form.
- `src/editor/panels/layout/LayoutPresetRow.tsx` - a row that is being renamed, in place of its name.
