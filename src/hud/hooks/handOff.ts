/** The post-preprocess editor handoff, pulled out of `useRecordingFlow.finish` into its own file
 *  (fix round 1, item 2, controller ruling 2026-09-02 - also keeps `useRecordingFlow.ts` under the
 *  project's 200-line cap) so the ordering invariant it exists for is independently testable with
 *  a fake `onEdit`, without rendering the hook or mocking IPC/Tauri.
 *
 *  **The bug this fixes.** `finish` used to call `onEdit?.(folder)` WITHOUT awaiting it, then its
 *  own `finally` reset `saving` on the very next tick - racing `App.tsx`'s `openEditor`, which
 *  runs its OWN async `setResizable`/`setSize(w,h)`/`center` sequence on the SAME window before
 *  switching views. Whichever Tauri `setSize` call resolved last silently won, so the editor could
 *  open bar-sized. It also re-armed `cam`'s `camOn && !saving` gate (item 4) for a moment while
 *  `Hud` was still mounted, flickering the webcam preview back on before the handoff completed.
 *
 *  **The fix.** `finish` now awaits this before deciding whether to reset `saving`: `true` (reset)
 *  when there's nothing to hand off to (`onEdit` wasn't provided - standalone mode), or `false`
 *  (LEAVE `saving` true) once `onEdit`'s own promise resolves - by then the caller has already
 *  finished resizing/switching views, so there's no window-size call left to race, and `Hud` is
 *  moments from unmounting anyway. A rejection propagates instead of being swallowed - `finish`'s
 *  `resetSaving` local starts `true` and is only flipped by a completed, successful call here, so
 *  a failed handoff still restores idle rather than leaving the bar stuck on "Saving…" forever. */
export async function handOff(onEdit: ((folder: string) => Promise<void>) | undefined, folder: string): Promise<boolean> {
  if (!onEdit) return true;
  await onEdit(folder);
  return false;
}
