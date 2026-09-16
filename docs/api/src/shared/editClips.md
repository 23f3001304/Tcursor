# src/shared/editClips.ts

TypeScript mirror of `src-tauri/src/edit/clip.rs`'s `Clip`, re-exported from `src/shared/edit.ts`. Keep in sync with the Rust side.

## Clip

```ts
export interface Clip {
  id: string;
  src_in_ms: number;
  src_out_ms: number;
  transition_in_ms: number;
}
```

One piece of the recording, in OUTPUT ORDER (the array order of `EditDoc.clips`) - mirrors Rust `Clip`.

- `id: string` - *stable identifier. A re-record may renumber a clip's DISPLAY name, but the id still resolves to the same source range.*
- `src_in_ms` / `src_out_ms: number` - *this clip's range in CLIP time (the recording's own clock, not output time).*
- `transition_in_ms: number` - *cross-dissolve INTO this clip from the previous one, in ms; `0` is a hard cut. Ignored on the first clip.*

### Used by

- `src/shared/edit.ts` - field `EditDoc.clips`, re-exports this type.
