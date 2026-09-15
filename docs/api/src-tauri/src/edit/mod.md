# src-tauri/src/edit/mod.rs

MODULE OVERVIEW: The `edit` module owns the complete lifecycle of a recording project's edit document (`edit.json`): data types, mutation logic, seeding from raw recording data, locking, and IPC command handlers. The layered stack: `model` defines the serializable type hierarchy, `api` provides the single mutation function `apply` and the read-only `metrics` summary, `effects` holds the spotlight effect-region ops that `api` delegates, `migrate` handles schema-version upgrades, `lock` is the shared per-project-folder write lock, `seed` builds the first `EditDoc` from raw recording artifacts and handles the load-or-create pattern (self-locking against `lock`), and `commands` exposes three Tauri IPC handlers that the frontend calls to read, mutate, and save the document. No I/O occurs in `model` or `api`; all disk access is confined to `seed` and `commands`.

## model

Serializable type hierarchy for `edit.json`: atomic edit types up to the root `EditDoc`, plus self-contained save/load helpers. Key items: `EditDoc` (root document with `trim`, `cuts`, `zooms`, `speed`, `layout`, `effects`, and `settings` fields), `EditDoc::save` and `EditDoc::load` (JSON round-trip), `Zoom` (timeline zoom event with id, span, target, scale, and easing), `ZoomTarget` enum (`Cursor` or `Fixed { x, y }`), `EffectRegion` + `EffectKind` (editable effect regions, e.g. Spotlight), `Trim`, `Cut`, `Speed`, `LayoutSeg`.

## lock

The shared per-project-folder write lock (`doc_lock`) guarding every `edit.json` write, wherever it originates - `edit::commands`' three IPC entry points AND `seed::load_or_seed`'s own internal seed/migrate/lift write (reached from ~8 other call sites across `ai`/`export`). See `docs/api/src-tauri/src/edit/lock.md` for the full ordering picture, including why it never inverts against `export::preview::session::WarmSlot`'s `gate` mutex.

## seed

Builds the first `EditDoc` from raw recording data (zoom regions and action log) so the editor opens in a state that matches today's exporter, then writes it as `edit.json`; on subsequent opens returns the existing file untouched. Holds the pure builders (`build_default`, `zooms_from_regions`, `layout_from_actions`, `actions_on_output_clock`) plus the load-or-create entry point (`load_or_seed`, self-locking against `edit::lock::doc_lock`) and the two-phase fast-path/precompute-then-lock machinery that keeps that lock cheap (`load_or_seed_locked`, `derive_seed_inputs`, `needs_seed_write`) - see `docs/api/src-tauri/src/edit/seed.md`.

## commands

Three Tauri IPC command handlers covering the full read-mutate-save lifecycle; the only code in the `edit` module that touches the Tauri command bus directly. Key items: `get_edit` (delegates fully to the self-locking `seed::load_or_seed`), `apply_edit_op` (takes `edit::lock::doc_lock` once, covering both `seed::load_or_seed_locked` and its own apply+save), `save_edit` (takes the same lock directly and overwrites `edit.json` with a caller-supplied doc for bulk frontend mutations).

## remap_doc

`remap_doc(doc, &TimeMap)`: every region list moved onto the output clock (durations and ids untouched, collapsed regions dropped, trim/cuts/speed consumed) before the renderer builds its tracks. Mirrored by `src/shared/math/remapDoc.ts`.
