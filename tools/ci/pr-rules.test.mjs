import { test } from "node:test";
import assert from "node:assert/strict";
import {
  MAX_SOURCE_LINES,
  docsPageOf,
  docsReport,
  isDocumented,
  isSource,
  parseNameStatus,
  parseNumstat,
  sizeReport,
} from "./pr-rules.mjs";

test("source files count towards the size, tests docs fixtures and lockfiles do not", () => {
  for (const p of [
    "src/editor/Editor.tsx",
    "src/editor/editor.css",
    "src-tauri/src/export/remap.rs",
    "src-tauri/src/export/fx/fx.wgsl",
    "setup/src-tauri/src/main.rs",
    "tools/linecount.mjs",
  ])
    assert.equal(isSource(p), true, p);
  for (const p of [
    "src/shared/math/remap.test.ts",
    "src-tauri/src/export/remap_tests.rs",
    "src-tauri/tests/manual_export.rs",
    "src/shared/math/remap.fixture.ts",
    "src/vite-env.d.ts",
    "docs/api/src/editor/Editor.md",
    "package-lock.json",
    "src-tauri/assets/cursors/arrow.png",
    "tools/ci/pr-rules.test.mjs",
  ])
    assert.equal(isSource(p), false, p);
});

test("a docs page mirrors the source path with the extension dropped", () => {
  assert.equal(docsPageOf("src-tauri/src/export/remap_spans.rs"), "docs/api/src-tauri/src/export/remap_spans.md");
  assert.equal(docsPageOf("src/editor/stage/useStageEngine.ts"), "docs/api/src/editor/stage/useStageEngine.md");
  assert.equal(docsPageOf("src/editor/inspectors/ClipInspector.tsx"), "docs/api/src/editor/inspectors/ClipInspector.md");
  assert.equal(isDocumented("src/editor/editor.css"), false);
  assert.equal(isDocumented("src-tauri/src/export/fx/fx.wgsl"), false);
});

test("numstat and name-status parse, binary rows count zero lines", () => {
  const rows = parseNumstat("10\t2\tsrc/a.ts\n-\t-\tsrc-tauri/assets/x.png\n3\t0\tdocs/api/src/a.md\n");
  assert.deepEqual(rows, [
    { path: "src/a.ts", lines: 12 },
    { path: "src-tauri/assets/x.png", lines: 0 },
    { path: "docs/api/src/a.md", lines: 3 },
  ]);
  assert.deepEqual(parseNameStatus("M\tsrc/a.ts\nA\tsrc/b.ts\nD\tsrc/c.ts\n"), [
    { status: "M", path: "src/a.ts" },
    { status: "A", path: "src/b.ts" },
    { status: "D", path: "src/c.ts" },
  ]);
});

test("the size rule sums source only and trips one line past the limit", () => {
  const under = sizeReport([
    { path: "src/a.ts", lines: MAX_SOURCE_LINES },
    { path: "src/a.test.ts", lines: 5000 },
    { path: "docs/api/src/a.md", lines: 900 },
  ]);
  assert.equal(under.total, MAX_SOURCE_LINES);
  assert.equal(under.over, false);
  const over = sizeReport([
    { path: "src/a.ts", lines: 250 },
    { path: "src-tauri/src/b.rs", lines: 151 },
  ]);
  assert.equal(over.total, 401);
  assert.equal(over.over, true);
  assert.deepEqual(
    over.counted.map((r) => r.path),
    ["src/a.ts", "src-tauri/src/b.rs"],
  );
});

test("a changed source file must bring its docs page, a new one must add it, a deleted one owes nothing", () => {
  const exists = (page) => page !== "docs/api/src/legacy.md";
  const r = docsReport(
    [
      { status: "M", path: "src/ok.ts" },
      { status: "M", path: "docs/api/src/ok.md" },
      { status: "M", path: "src/forgot.ts" },
      { status: "A", path: "src-tauri/src/new.rs" },
      { status: "D", path: "src/gone.ts" },
      { status: "M", path: "src/legacy.ts" },
      { status: "M", path: "src/a.test.ts" },
      { status: "M", path: "src/editor/editor.css" },
    ],
    exists,
  );
  assert.deepEqual(r.missing, [
    { path: "src/forgot.ts", page: "docs/api/src/forgot.md", added: false },
    { path: "src-tauri/src/new.rs", page: "docs/api/src-tauri/src/new.md", added: true },
  ]);
  assert.deepEqual(r.legacy, [{ path: "src/legacy.ts", page: "docs/api/src/legacy.md" }]);
});
