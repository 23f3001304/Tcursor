# src-tauri/src/ai/plan/schema.rs

The shape of a propose pass, and the constants the mapping validates against. No logic and no I/O: this is the contract `mapping.rs` fills, `run.rs` returns and `src/shared/aiRun.ts` mirrors on the wire.

## NEW_ID

```rust
pub const NEW_ID: &str = "$new"
```

The id a follow-up op carries where the id of the region the PREVIOUS op created belongs. Rust cannot know it: `AddZoomFull` allocates the id inside `edit::ops::api::apply`, and predicting it here would couple this module to id allocation and be wrong under any concurrent apply (A2). The frontend substitutes the real id by diffing the doc across the apply.

## PRE_ROLL_MS

```rust
pub const PRE_ROLL_MS: u32 = 300
```

How far before a click a snapped zoom starts, so the camera is already moving when the click lands rather than reacting to it.

## SNAP_MS

```rust
pub const SNAP_MS: u32 = 1_000
```

A model-supplied time within this of a real click IS that click. Wide enough to absorb the model's own rounding, narrow enough that an unrelated click two seconds away is not claimed as the reason.

## WHY_MAX

```rust
pub const WHY_MAX: usize = 120
```

The sheet gives a reason one line, so a reason gets one line's worth of characters.

## MIN_SCALE

```rust
pub const MIN_SCALE: f32 = 1.2
```

Below this a zoom is not visible as a zoom, just a drift.

## MAX_SCALE

```rust
pub const MAX_SCALE: f32 = 3.0
```

Above this a proxy-resolution recording is visibly soft.

## RENDERABLE_KINDS

```rust
pub const RENDERABLE_KINDS: [ProposalKind; 6]
```

Kinds the editor can actually RENDER, and therefore the only kinds worth proposing. The plan's A3 note gated `Cut` and `Speed` out because `doc.cuts` / `doc.speed` reached no renderer; the time-remap milestone landed and they now render in the export, the audio and the preview, so all six are offered from the start.

## ProposalKind

```rust
pub enum ProposalKind { Zoom, Layout, Spotlight, Trim, Cut, Speed }
```

Serialized `snake_case`, which is the wire name the prompt asks for and `AiProposalKind` mirrors in TypeScript.

### parse

```rust
pub(crate) fn parse(s: &str) -> Option<Self>
```

The wire name back to a kind, trimmed and lowercased, `None` for anything this editor has never heard of. A `None` drops that item alone; the rest of the plan stands.

## AiProposal

```rust
pub struct AiProposal {
    pub id: String, pub kind: ProposalKind, pub why: String,
    pub at_ms: u32, pub dur_ms: u32,
    pub rect: Option<[f32; 4]>, pub ops: Vec<EditOp>,
}
```

One proposed edit, reviewable on its own.

- `id` - `p0`, `p1`, ... assigned in final time order, the handle the sheet toggles by.
- `why` - one line, present tense, saying what is happening at that moment. Never what the edit does: the user can see that.
- `at_ms` / `dur_ms` - where it sits on the OUTPUT clock. A trim is `(0, 0)`: it is a property of the clip's ends, not a moment in it.
- `rect` - the region the model named, for the stage outline ONLY. It never reaches `EditDoc`, because `ZoomTarget` is a point and `EffectRegion` has no position at all (A1, A4). `None` when the model named none, or named one this file refused.
- `ops` - what applying it actually does, in order. A SEQUENCE rather than one op because `AddZoomFull` hardcodes `ZoomTarget::Cursor` and there is no add-with-target op, so aiming needs a follow-up `UpdateZoom` carrying `NEW_ID` (A2).

## AiRun

```rust
pub struct AiRun {
    pub model: String, pub vision: bool,
    pub frames: usize, pub elapsed_ms: u64,
    pub proposals: Vec<AiProposal>,
}
```

One propose pass, and everything the sheet header needs to be honest about it: which model answered, whether it could see, how many frames it ACTUALLY received (not how many were asked for), and how long the wait was. An empty `proposals` is a real answer, not a failure, and the sheet says so in words.

## ClickAt

```rust
pub struct ClickAt { pub t_ms: u32, pub x: f32, pub y: f32 }
```

One click on the OUTPUT clock with its point as 0..1 fractions of the screen content - exactly what `ZoomTarget::Fixed` stores, so a snapped zoom can borrow it without converting anything. Built once per run in `ai::run::click_points`.
