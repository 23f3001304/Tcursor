// The two pull request rules CONTRIBUTING.md states, enforced by .github/workflows/ci.yml:
//   size: a pull request changes at most MAX_SOURCE_LINES lines of source
//   docs: a changed source file brings its docs/api page along in the same pull request
// Tests, docs pages, lockfiles, fixtures and assets never count towards the size.
// Usage: node tools/ci/pr-rules.mjs <base-ref>      labels come from the PR_LABELS env (comma separated)
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const MAX_SOURCE_LINES = 400;
export const LABEL_LARGE = "large-pr";
export const LABEL_NO_DOCS = "docs-not-needed";

const SOURCE = [
  /^src\/.+\.(ts|tsx|css)$/,
  /^src-tauri\/src\/.+\.(rs|wgsl)$/,
  /^setup\/src-tauri\/src\/.+\.rs$/,
  /^setup\/ui\/.+\.(js|css|html)$/,
  /^tools\/.+\.(mjs|ts|ps1)$/,
];
const DOCUMENTED = [/^src\/.+\.(ts|tsx)$/, /^src-tauri\/src\/.+\.rs$/];
const NEVER = [/\.test\.(ts|tsx|mjs)$/, /_tests\.rs$/, /^src-tauri\/tests\//, /\.d\.ts$/, /\.fixture\.ts$/];

const matches = (list, path) => list.some((re) => re.test(path));
export const isSource = (path) => matches(SOURCE, path) && !matches(NEVER, path);
export const isDocumented = (path) => matches(DOCUMENTED, path) && !matches(NEVER, path);
export const docsPageOf = (path) => `docs/api/${path.replace(/\.[^./]+$/, "")}.md`;

export function parseNumstat(text) {
  return text
    .split("\n")
    .map((line) => line.split("\t"))
    .filter((cols) => cols.length === 3)
    .map(([added, deleted, path]) => ({
      path,
      lines: (Number(added) || 0) + (Number(deleted) || 0),
    }));
}

export function parseNameStatus(text) {
  return text
    .split("\n")
    .map((line) => line.split("\t"))
    .filter((cols) => cols.length === 2)
    .map(([status, path]) => ({ status: status[0], path }));
}

export function sizeReport(rows, max = MAX_SOURCE_LINES) {
  const counted = rows.filter((r) => isSource(r.path)).sort((a, b) => b.lines - a.lines);
  const total = counted.reduce((sum, r) => sum + r.lines, 0);
  return { total, max, over: total > max, counted };
}

export function docsReport(changes, pageExists) {
  const touched = new Set(changes.map((c) => c.path));
  const missing = [];
  const legacy = [];
  for (const c of changes) {
    if (c.status === "D" || !isDocumented(c.path)) continue;
    const page = docsPageOf(c.path);
    if (touched.has(page)) continue;
    if (c.status !== "A" && !pageExists(page)) legacy.push({ path: c.path, page });
    else missing.push({ path: c.path, page, added: c.status === "A" });
  }
  return { missing, legacy };
}

function git(args) {
  return execFileSync("git", args, { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
}

function main() {
  const base = process.argv[2];
  if (!base) {
    console.error("usage: node tools/ci/pr-rules.mjs <base-ref>");
    process.exit(2);
  }
  const labels = (process.env.PR_LABELS ?? "").split(",").map((l) => l.trim());
  const range = `${base}...HEAD`;
  const size = sizeReport(parseNumstat(git(["diff", "--no-renames", "--numstat", range])));
  const docs = docsReport(parseNameStatus(git(["diff", "--no-renames", "--name-status", range])), existsSync);
  let failed = false;

  console.log(`Source lines changed: ${size.total} of ${size.max} allowed`);
  for (const r of size.counted.slice(0, 15)) console.log(`  ${String(r.lines).padStart(5)}  ${r.path}`);
  if (size.over && labels.includes(LABEL_LARGE)) {
    console.log(`Over the limit, accepted by the "${LABEL_LARGE}" label.`);
  } else if (size.over) {
    failed = true;
    console.log(`::error::This pull request changes ${size.total} lines of source; the limit is ${size.max}. Split it into smaller pull requests, or ask a maintainer for the "${LABEL_LARGE}" label.`);
  }

  for (const l of docs.legacy) console.log(`  note: ${l.path} has no docs page yet (${l.page}); consider adding one`);
  if (docs.missing.length > 0 && labels.includes(LABEL_NO_DOCS)) {
    console.log(`${docs.missing.length} source file(s) changed without their docs page, accepted by the "${LABEL_NO_DOCS}" label.`);
  } else {
    for (const m of docs.missing) {
      failed = true;
      const what = m.added ? "is new and needs its docs page" : "changed but its docs page did not";
      console.log(`::error file=${m.path}::${m.path} ${what}: ${m.page}`);
    }
  }
  if (docs.missing.length === 0) console.log("Docs pages: every changed source file brought its page along.");
  process.exit(failed ? 1 : 0);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) main();
