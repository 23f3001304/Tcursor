# src-tauri/src/actions/matcher.rs

Parses user-configured hotkey strings into typed chord descriptors and matches live key events against an armed set, emitting `ActionEvent` values on press and (for hold-type actions) on release. Pure data-transformation with no I/O and no threading - all state lives in `ActionMatcher`'s two vecs.

## Mods

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Mods { pub ctrl: bool, pub alt: bool, pub shift: bool }
```

Snapshot of the three modifier keys at the moment of a key event. `Default` yields all-false (no modifiers held). *Why derived `PartialEq`:* chord matching requires all three bits to agree exactly - partial modifier matching would cause false positives when a user holds an extra modifier.

- `ctrl` - whether VK_CONTROL (0x11) is held. *Why:* distinguishes `Ctrl+Z` from bare `Z`.
- `alt` - whether VK_MENU (0x12) is held. *Why:* all default TCursor chords use `Ctrl+Alt` to avoid clashing with application shortcuts.
- `shift` - whether VK_SHIFT (0x10) is held. *Why:* makes the chord namespace larger so future bindings can avoid conflicts.

### Used by

- `src-tauri/src/actions/keyboard.rs` - `cur_mods()` constructs a `Mods` each poll cycle and compares it against each arm's chord.
- `src-tauri/src/actions/matcher.rs` - `KeyChord::matches` and `ActionMatcher::on_key` compare live `Mods` against the chord's expected `Mods`.

## KeyChord

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyChord { pub mods: Mods, pub vk: u32 }
```

A fully parsed hotkey: a `Mods` snapshot plus one Win32 virtual-key code for the main (non-modifier) key. `Copy` because it is stored and compared frequently with no heap involvement.

- `mods` - the required modifier state. *Why stored rather than inlined:* `matches` can compare the whole `Mods` struct in one `==` call.
- `vk` - the VK code for the main key (A-Z: `0x41-0x5A`, 0-9: `0x30-0x39`). *Why Win32 VKs:* `GetAsyncKeyState` uses these codes; for ASCII letters and digits the VK equals the ASCII uppercase byte, so no lookup table is needed.

### Used by

- `src-tauri/src/actions/matcher.rs` - `Arm` stores a `KeyChord`; `ActionMatcher::on_key` and `arming_from_settings` create and compare them.
- `src-tauri/src/actions/keyboard.rs` - poll loop reads `arm.chord.vk` and passes live `Mods` to `arm.chord.mods ==` comparison.

## KeyChord::parse

```rust
pub fn parse(s: &str) -> Option<KeyChord>
```

Parses a `'+'`-separated string such as `"Ctrl+Alt+Z"` or `"ctrl+alt+1"` (case-insensitive) into a `KeyChord`.

### Inputs

- `s: &str` - the raw hotkey string from user settings. *Why `Option` return:* invalid or unknown strings must silently yield `None` so `arming_from_settings` can skip them without aborting the recording.

### Implementation

1. Split on `'+'`, trim each token, reject empty tokens immediately.
2. Match each token case-insensitively: `"ctrl"` / `"control"` set `mods.ctrl`; `"alt"` sets `mods.alt`; `"shift"` sets `mods.shift`; anything else is passed to `main_key_vk`.
3. `main_key_vk` accepts exactly one ASCII letter or digit character; returns `None` for multi-character tokens or unrecognized characters. *Why single-char restriction:* multi-character tokens that are not modifier aliases are typos (e.g. `"Ctr"`) and should fail rather than bind to an unexpected key.
4. If a second main key is seen, return `None`. *Why:* two simultaneous non-modifier keys cannot be reliably detected by polling-based `GetAsyncKeyState`.
5. If no main key was found after all tokens, return `None` via `vk?`.

### Returns

`Some(KeyChord)` with the parsed modifier flags and VK code, or `None` if any token is invalid, there is no main key, or there are two main keys.

### Behaviors

- `parses_letter_and_digit_case_insensitive` - `"Ctrl+Alt+Z"` and `"ctrl+ALT+1"` both parse correctly.
- `rejects_chords_without_a_single_main_key` - modifiers-only, unknown tokens, two main keys, and empty string all return `None`.

## KeyChord::matches

```rust
pub fn matches(&self, vk: u32, mods: Mods) -> bool
```

Returns `true` iff `vk` equals `self.vk` and `mods` equals `self.mods` exactly.

### Inputs

- `vk: u32` - the VK code from the live key event. *Why:* separating VK and mods lets callers pass them independently from whatever source provides raw key data.
- `mods: Mods` - the modifier snapshot at the moment of the event. *Why:* exact equality means a chord only fires when all three bits match - no spurious fire if the user holds an extra modifier.

### Behaviors

- `matches_requires_exact_modifiers` - `Ctrl+Alt+Z` does not match when Alt is not held, and does not match `X` with the same mods.

## Arm

```rust
pub struct Arm { pub chord: KeyChord, pub on_down: ActionKind, pub on_up: Option<ActionKind> }
```

Associates a parsed `KeyChord` with the `ActionKind` emitted on press and optionally on release.

- `chord` - the parsed hotkey this arm responds to. *Why stored by value:* `KeyChord` is `Copy`; no indirection needed.
- `on_down` - the action emitted when the chord goes down. *Why always present:* every arm must do something on press; an arm with no `on_down` purpose would never be added.
- `on_up` - `Some(kind)` for hold-type actions (zoom, spotlight, video FX) so a matched release emits the corresponding end-event. `None` for layout toggles, which take effect on press only. *Why `Option`:* avoids a separate arm type for one-shot vs held actions.

### Used by

- `src-tauri/src/actions/matcher.rs` - `ActionMatcher` stores `Vec<Arm>`; `arming_from_settings` builds the table.
- `src-tauri/src/actions/keyboard.rs` - `KeyboardTracker::start` receives `Vec<Arm>` to determine which VKs to poll.

## ActionMatcher

```rust
pub struct ActionMatcher { arms: Vec<Arm>, held: Vec<usize> }
```

Stateful matcher that tracks which arm indices are currently held. Used by hook-based key event sources (not by `KeyboardTracker`, which uses polling and its own `active` array).

- `arms` - the armed chord table, built by `arming_from_settings`. *Why stored here:* `on_key` needs to look up `on_up` by arm index to emit the correct release event.
- `held` - indices into `arms` for chords currently pressed. *Why indices rather than VKs:* the arm index gives direct access to `on_up` without a second search; and pairing by VK on release (ignoring current modifier state) is safe because the arm index already encodes which chord was matched on press.

### Used by

- Tests in `matcher.rs` exercise all matching paths directly via `ActionMatcher::new(arming_from_settings(...))`.

## ActionMatcher::new

```rust
pub fn new(arms: Vec<Arm>) -> Self
```

Constructs a matcher with an empty `held` set. `arms` is typically produced by `arming_from_settings`.

### Inputs

- `arms: Vec<Arm>` - the full set of chords to recognize. *Why taken by value:* the matcher owns the table for the lifetime of a recording session; no shared reference is needed.

### Returns

`Self` with `held: Vec::new()`.

## ActionMatcher::on_key

```rust
pub fn on_key(&mut self, down: bool, vk: u32, mods: Mods, t: u32) -> Option<ActionEvent>
```

Processes one raw key event and returns the resulting `ActionEvent` if any.

### Inputs

- `down: bool` - `true` for key-down, `false` for key-up. *Why:* the two directions require different matching logic (full chord vs VK-only) and different arms (`on_down` vs `on_up`).
- `vk: u32` - the Win32 VK code of the key that changed state. *Why:* the main key VK is the primary identifier for both press matching and release pairing.
- `mods: Mods` - current modifier state. *Why only used on press:* when the user releases the main key they may have already let go of modifiers; pairing on release by VK alone ensures the `End` event is always emitted even after modifier keys are released first.
- `t: u32` - session-relative timestamp in milliseconds. *Why:* timestamps in the emitted `ActionEvent` must match the same epoch as mouse events so the AI director can correlate them.

### Implementation

1. **Press path** (`down = true`): find the first arm where `chord.matches(vk, mods)`; return `None` if none. If the arm index is already in `held`, return `None` to suppress OS auto-repeat. Otherwise push the index into `held` and return `Some(ActionEvent { t, kind: arm.on_down })`.
2. **Release path** (`down = false`): search `held` for any entry whose arm's `chord.vk == vk` (modifier-agnostic). Return `None` if not found (unmatched release). Otherwise `remove` that entry from `held` and return `arm.on_up.map(|kind| ActionEvent { t, kind })` - `None` for layout arms, `Some` for hold arms.

### Returns

`Some(ActionEvent)` when the event triggers a meaningful action; `None` for unrecognized keys, auto-repeat, or releases of one-shot arms.

### Behaviors

- `layout_emits_once_and_suppresses_autorepeat` - a second down event while held returns `None`; release of a layout arm (no `on_up`) also returns `None`.
- `zoom_hold_pairs_start_then_end_even_if_mods_released_first` - releasing the main key after the modifiers are already up still emits `ZoomHoldEnd`.
- `spotlight_hold_pairs_start_then_end_even_if_mods_released_first` - same pairing guarantee for spotlight.
- `unarmed_key_is_ignored` - a VK not in any arm returns `None` on press.

## arming_from_settings

```rust
pub fn arming_from_settings(h: &HotkeySettings) -> Vec<Arm>
```

Converts a `HotkeySettings` struct into the armed-chord table consumed by `KeyboardTracker::start` and `ActionMatcher::new`.

### Inputs

- `h: &HotkeySettings` - the user's configured hotkey strings. *Why by reference:* settings are read-only; the function only needs to parse the string fields.

### Implementation

1. Allocate an empty `arms` vec.
2. Call the private `push_arm` helper for each configured hotkey field. `push_arm` calls `KeyChord::parse` and silently skips the entry if parsing returns `None`. *Why silent skip:* an unparseable chord string in settings should not prevent the recording from starting; the action is simply unbound.
3. Arms added in order: `zoom_hold` (ZoomHoldStart/ZoomHoldEnd), `spotlight_hold` (SpotlightHoldStart/SpotlightHoldEnd), `video_fx_hold` (VideoFxHoldStart/VideoFxHoldEnd), and five layout keys (`layout_screen`, `layout_camera`, `layout_presenter`, `layout_screen_only`, `layout_camera_only`) each with `on_up: None`.

### Returns

`Vec<Arm>` with one entry per successfully parsed hotkey setting; entries for unparseable strings are absent.

### Behaviors

- `arming_includes_spotlight_hold_arm` - with default settings the result contains an arm for `SpotlightHoldStart` bound to `Ctrl+Alt+S` with `on_up = Some(SpotlightHoldEnd)`.

### Used by

- `src-tauri/src/session/record/recorder.rs` - called once at recording start: `KeyboardTracker::start(arming_from_settings(&snap.hotkeys))`.
