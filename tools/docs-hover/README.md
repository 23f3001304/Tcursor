# TCursor Docs Hover

A tiny VS Code extension that is TCursor's own "project IntelliSense". It shows
rich Markdown documentation for our functions **on hover**, read from an external
store under `docs/api/`, so the source code stays free of docstrings and the
hard 200-line-per-file budget is untouched.

## How it works

- **Doc store:** `docs/api/<source-path>.md` mirrors the source tree. Example:
  `src-tauri/src/edit/api.rs` is documented by `docs/api/src-tauri/src/edit/api.md`.
- Inside each doc file, one `## symbol_name` section per function/type. The body
  is plain Markdown (params, examples, gotchas), as detailed as you like.
- On hover, the extension asks the language server **where the symbol is defined**
  (`vscode.executeDefinitionProvider`), finds that file's mirrored `.md`, pulls the
  matching `## name` section, and renders it under the native type tooltip.
- Resolving by definition (not the bare word) means two different `apply()`
  functions in different files show their own docs. Methods can be documented as
  `## Type::method`; hovering `method` still matches.

## Run it

```sh
cd tools/docs-hover
npm install
npm run compile      # tsc -> out/
```

Then in VS Code open the repo, open `tools/docs-hover` once and press **F5** to
launch an Extension Development Host with the extension loaded. Hover any
documented symbol in a `.rs` / `.ts` / `.tsx` file.

To install it permanently, package with `npx vsce package` and
`code --install-extension tcursor-docs-hover-0.1.0.vsix`.

## Keep docs honest

```sh
npm test         # unit tests for the pure helpers
npm run validate # every `## section` must resolve to a real symbol (exits 1 on drift)
```

Wire `npm --prefix tools/docs-hover run validate` into precommit/CI so a renamed
or deleted function with a stale doc section fails the build.

## Limitation

This only works **inside VS Code with the extension installed**. The docs do not
appear in `cargo doc`, docs.rs, GitHub hovers, or other editors. That is the
deliberate trade for keeping the source code clean.
