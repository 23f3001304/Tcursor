# src/editor/stage/stageCursor.ts

What the stage draws for the `System` cursor setting: the captured OS-cursor layer when the recording has one, and a plain-OS fallback when it does not. Split back out of `Stage.tsx` - it is the preview's half of a rule the export also implements, and it belongs next to that argument rather than under a component.

## stageCursor

```ts
export function stageCursor(cursor: CursorSettings, osCursorInVideo: boolean, cursorLayer: CursorLayerDto | null): { captured, plainOs, effCursor }
```

What the stage actually draws for the cursor setting. "System" means the captured OS-cursor layer when the recording has one (`captured`); a recording that baked no cursor into its pixels and captured no layer has nothing original to show (`plainOs`), so it is redrawn as a plain Enhanced pointer with no bounce, no trail, no motion tilt and no glass back (`effCursor`), and `Stage` drops the cursor-kind track so the sprite never changes shape. That list is the preview's half of Rust `CursorSettings::plain_os` - the same "no fake polish a real OS cursor does not have" rule `smoothness_at` / `idealize_at` / `tilt_at` apply on the export side, so anything added to one must be added here too.

### The "System" cursor: captured layer first, plain-OS fallback second

Mirrors the renderer's own two-way split (`export::cursor::captured::draws_captured`, then `cursorset::draw`):

```ts
const captured = cursor.style === "system" ? cursorLayer : null;
const plainOs = cursor.style === "system" && !osCursorInVideo && !captured;
```

**Captured (the normal case now).** Any recording made since screen capture went cursor-free has a layer, so `"system"` composites the REAL bitmap that was on screen - `captured` is handed to `useCursorSprites`, lands on `DrawCursor.captured`, and `drawCursorSprite` takes its captured branch and returns before the synthetic gate. The style is deliberately left as `"system"` here (no effective-style substitution), because a non-null `captured` IS the gate.

**Plain-OS (pre-layer recordings only).** Below is the older fallback, now reachable only when there is no layer to composite.

### Plain-OS cursor fallback

`cursor.style === "system" && !osCursorInVideo && !captured` is the case the renderer calls plain-OS: the doc asks for the system cursor, but the video was recorded in `Enhanced` (or `Hidden`) BEFORE the cursor layer existed, so it has neither a baked cursor nor a captured one. Rust re-creates it from the recorded path (`cursorset::draw`); Stage mirrors that decision for the canvas preview **without any new drawing code**, by feeding `useSyncRefs` an *effective* cursor instead of the raw prop:

- `style: "enhanced"` - flips on the existing `drawCursorSprite` gate, which is a plain style check.
- an empty `cursorKinds` array - `cursorAt` then always resolves to `"arrow"`, matching the renderer's always-Arrow rule.
- `click_bounce: false`, `motion_blur: 0` - the polish a real OS cursor doesn't have.

*Why nothing here handles the raw path:* preview cursor positions come from `camera_track`, which Rust computes with `CursorSettings::smoothness_at` - already the raw path in this mode. Only the sprite-side knobs need mirroring.

*Why the transform lives in Stage rather than in `useCompositeLoop` or `cursorPreview`:* both of those consume a `CursorSettings`/kinds pair that Stage already owns, so expressing the fallback as "effective settings" keeps the loop and the draw function completely unchanged.
