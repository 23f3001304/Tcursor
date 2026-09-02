# src/hud/hooks/handOff.ts

The post-preprocess editor handoff, pulled out of `useRecordingFlow.finish` into its own file (fix round 1, item 2, controller ruling 2026-09-02 - also keeps `useRecordingFlow.ts` under the project's 200-line cap) so the ordering invariant it exists for is independently testable with a fake `onEdit`, without rendering the hook or mocking IPC/Tauri.

## handOff

```ts
export async function handOff(onEdit: ((folder: string) => Promise<void>) | undefined, folder: string): Promise<boolean>
```

### The bug this fixes

`finish` used to call `onEdit?.(folder)` WITHOUT awaiting it, then its own `finally` reset `saving` to `false` on the very next tick - `useHudWindowSize` (item 3) reacts to that by shrinking the HUD's OS window back to the idle 980x132 size. But `onEdit` (`App.tsx`'s `openEditor`, see `App.md`) is ALSO mid-flight at that exact moment, running its own async `setResizable`/`setSize(w,h)`/`center` sequence on the SAME window before switching `view` to `"editor"`. Two independent `setSize` calls on one window, racing - whichever Tauri call resolves last silently wins, so the editor could open bar-sized.

It also caused a secondary bug: `saving` flipping `false` while `Hud` was still mounted (view hadn't switched to `"editor"` yet) re-armed `cam`'s `camOn && !saving` gate (item 4), so the webcam preview would flicker back on for the handoff's duration before `Hud` finally unmounted.

### The fix

`useRecordingFlow.finish` now AWAITS this function before its `finally` decides whether to reset `saving`.

### Inputs

- `onEdit: ((folder: string) => Promise<void>) | undefined` - `Hud`'s own `onEdit` prop, forwarded as-is.
- `folder: string` - the finished project's folder.

### Returns

- `true` ("reset `saving`") when there's nothing to hand off to (`onEdit` wasn't provided - standalone mode).
- `false` ("leave `saving` true") once `onEdit`'s own promise resolves - by then the caller has already finished resizing/switching views, so there is no window-size call left for anything in `finish` to race, and `Hud` is moments from unmounting anyway with `saving` still `true`, which is honest (it never got a chance to become not-saving before the editor took over).
- A rejection PROPAGATES to the caller instead of being swallowed here - `finish`'s own `finally` still resets `saving` on that path, since its `resetSaving` local starts `true` and is only flipped by a completed, successful call to this function (see `useRecordingFlow.md`'s `finish` notes).

### Used by

- `src/hud/hooks/useRecordingFlow.ts` - `finish()`, awaited as `resetSaving = await handOff(onEdit, res.folder)`.
- `src/hud/hooks/handOff.test.ts` - direct unit coverage with a fake `onEdit` (a controllable deferred promise), pinning that `handOff` does not resolve until `onEdit`'s own promise settles - the whole point of the fix.
