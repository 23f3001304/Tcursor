// Reports every source file over its kind's line budget. It only reports (exit 0) unless --check
// is given, which is what CI passes: then any file over its budget fails the run (exit 1).
// Usage: node tools/linecount.mjs [--all] [--check]   (--all lists every file with its budget)
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const BUDGETS = {
  rustLogic: 280,
  rustTests: 320,
  rustIntegration: 280,
  ts: 200,
  tsTests: 220,
};
// Declaration-only files: one long enum or table, nothing to split by responsibility.
const EXEMPT = new Set([
  "src-tauri/src/edit/ops/api.rs",
  "src-tauri/src/export/cursor/pack/cursorset.rs",
  "src/hud/components/icons.tsx",
]);

function kindOf(rel) {
  if (rel.startsWith("src-tauri/tests/")) return "rustIntegration";
  if (rel.endsWith("_tests.rs")) return "rustTests";
  if (rel.endsWith(".rs")) return "rustLogic";
  if (/\.test\.tsx?$/.test(rel)) return "tsTests";
  if (/\.tsx?$/.test(rel) && !rel.endsWith(".d.ts")) return "ts";
  return null;
}

function* walk(dir) {
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "target" || name === "dist") continue;
    const p = join(dir, name);
    if (statSync(p).isDirectory()) yield* walk(p);
    else yield p;
  }
}

const all = process.argv.includes("--all");
const over = [];
const totals = {};
for (const root of ["src-tauri/src", "src-tauri/tests", "src"]) {
  for (const p of walk(join(ROOT, root))) {
    const rel = relative(ROOT, p).replace(/\\/g, "/");
    const kind = kindOf(rel);
    if (!kind) continue;
    const lines = readFileSync(p, "utf8").split("\n").length - 1;
    totals[kind] = (totals[kind] ?? 0) + 1;
    const budget = BUDGETS[kind];
    if (all) console.log(`${String(lines).padStart(5)} / ${budget}  ${rel}`);
    if (lines > budget && !EXEMPT.has(rel)) over.push({ rel, lines, budget, kind });
  }
}
over.sort((a, b) => b.lines - a.lines);
for (const o of over) console.log(`OVER ${String(o.lines).padStart(4)} > ${o.budget}  ${o.rel}`);
const counts = Object.entries(totals).map(([k, n]) => `${k} ${n}`).join(", ");
console.log(`${over.length} file(s) over budget (${counts}); exempt: ${[...EXEMPT].join(", ")}`);
if (process.argv.includes("--check") && over.length > 0) process.exit(1);
