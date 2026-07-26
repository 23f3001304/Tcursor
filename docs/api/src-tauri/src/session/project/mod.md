# src-tauri/src/session/project/mod.rs

MODULE OVERVIEW: The `.tcursor` project format - a small JSON manifest written into a recording's own folder (never a zip/copy of the multi-GB video: the folder itself IS the project). `manifest` owns the `ProjectManifest` type and its save/load/fallback logic; `recents` persists the small "recently opened" list next to `settings::store`'s `config.json`; `commands` exposes `open_project`, `list_recent_projects`, and the cold-start file-association plumbing (`get_launch_project`, `LaunchProject`, `launch_project_from_argv`) as Tauri commands / managed state.

## manifest

Submodule (`project/manifest.rs`). `ProjectManifest` (`version`, `created_unix_ms`, `source_w`/`source_h`, `app_version`, `preprocessed`) plus `save`/`load`/`new`/`load_or_default`. Written at `stop_recording` with `preprocessed: false`; `load_or_default` is the back-compat seam - a missing or corrupt manifest resolves to a synthesized "unknown source" default rather than an error. Full per-symbol docs in `project/manifest.md`.

## recents

Submodule (`project/recents.rs`). `RecentProject` (`folder`, `name`, `opened_unix_ms`) plus `list`/`touch`, persisted as `<config-dir>/TCursor/recents.json`, most-recently-opened first, capped at 10 entries. Full per-symbol docs in `project/recents.md`.

## commands

Submodule (`project/commands.rs`). The Tauri command surface: `open_project` (native `*.tcursor` file dialog -> containing folder), `list_recent_projects`, `get_launch_project` (reads the cold-start `LaunchProject` managed state), plus the shared pure core `folder_from_manifest_path` and `launch_project_from_argv`. Full per-symbol docs in `project/commands.md`.
