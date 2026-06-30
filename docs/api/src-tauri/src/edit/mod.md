# src-tauri/src/edit/mod.rs

MODULE OVERVIEW: The `edit` module owns the complete lifecycle of a recording project's edit document (`edit.json`): data types, mutation logic, seeding from raw recording data, and IPC command handlers. The five submodules form a layered stack: `model` defines the serializable type hierarchy, `api` provides the single mutation function `apply` and the read-only `metrics` summary, `effects` holds the spotlight effect-region ops that `api` delegates, `seed` builds the first `EditDoc` from raw recording artifacts and handles the load-or-create pattern, and `commands` exposes three Tauri IPC handlers that the frontend calls to read, mutate, and save the document. No I/O occurs in `model` or `api`; all disk access is confined to `seed` and `commands`.

## model

Serializable type hierarchy for `edit.json`: atomic edit types up to the root `EditDoc`, plus self-contained save/load helpers. Key items: `EditDoc` (root document with `trim`, `cuts`, `zooms`, `speed`, `layout`, `effects`, and `settings` fields), `EditDoc::save` and `EditDoc::load` (JSON round-trip), `Zoom` (timeline zoom event with id, span, target, scale, and easing), `ZoomTarget` enum (`Cursor` or `Fixed { x, y }`), `EffectRegion` + `EffectKind` (editable effect regions, e.g. Spotlight), `Trim`, `Cut`, `Speed`, `LayoutSeg`.

## seed

Builds the first `EditDoc` from raw recording data (zoom regions and action log) so the editor opens in a state that matches today's exporter, then writes it as `edit.json`; on subsequent opens returns the existing file untouched. Key items: `load_or_seed` (load-or-create entry point used by `commands` and `ai::commands`), `zooms_from_regions` (converts `ZoomRegion` slice to `Vec<Zoom>` with stable ids and `Fixed` anchors), `layout_from_actions` (converts `SetLayout` action track to consecutive `LayoutSeg` entries covering the full clip duration).

## api

Pure business logic for mutating and measuring an `EditDoc`; the single write point for all document mutations. Key items: `apply` (dispatches an `EditOp` variant to mutate `doc` in place), `metrics` (derives a read-only `Metrics` summary from the current doc state), `EditOp` enum (all editor operations: `AddZoom`, `AddZoomFull`, `UpdateZoom`, `RemoveZoom`, `SetTrim`, `AddCut`, `SetSpeed`, `SetLayoutSeg`, `AddEffect`, `UpdateEffect`, `RemoveEffect`), `Metrics` struct (`duration_ms`, `kept_ms`, `zoom_count`, `cut_count`). The effect ops are delegated to `effects::apply_effect`.

## effects

Effect-region edit ops (`add/update/remove_effect`), split out of `api.rs` so each file stays under the size limit. Key item: `apply_effect` - `api::apply` delegates the three effect-op variants here; `AddEffect` generates an `e`-prefixed id, `UpdateEffect` patches start/end, `RemoveEffect` drops by id.

## commands

Three Tauri IPC command handlers covering the full read-mutate-save lifecycle; the only code in the `edit` module that touches the Tauri command bus or the filesystem directly. Key items: `get_edit` (loads or seeds the `EditDoc` for a project folder), `apply_edit_op` (applies one `EditOp`, saves, and returns the updated doc), `save_edit` (overwrites `edit.json` with a caller-supplied doc for bulk frontend mutations).
