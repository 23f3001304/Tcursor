// Drift guard: every `## section` in docs/api must resolve to a real symbol in
// the mirrored source file. Run from the repo root: `node out/validate.js <root>`
// (the npm `validate` script passes ../.. so the repo root is the default).
import * as fs from "fs";
import * as path from "path";
import { definesSymbol, parseDoc, sourceCandidatesForDoc } from "./pure";

/** Recursively collect every .md file under a directory. */
function walk(dir: string): string[] {
  const out: string[] = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) out.push(...walk(p));
    else if (e.name.endsWith(".md")) out.push(p);
  }
  return out;
}

function main(): void {
  const root = path.resolve(process.argv[2] ?? ".");
  const apiRoot = path.join(root, "docs", "api");
  if (!fs.existsSync(apiRoot)) { console.log("docs-hover: no docs/api, nothing to validate"); return; }

  const problems: string[] = [];
  for (const docPath of walk(apiRoot)) {
    const rel = path.relative(root, docPath);
    const source = sourceCandidatesForDoc(root, docPath).find((c) => fs.existsSync(c));
    if (!source) { problems.push(`orphan doc (no source file): ${rel}`); continue; }
    const text = fs.readFileSync(source, "utf8");
    for (const heading of parseDoc(fs.readFileSync(docPath, "utf8")).keys()) {
      const symbol = heading.split("::").pop() ?? heading;
      if (!definesSymbol(text, symbol)) {
        problems.push(`${rel}: "## ${heading}" has no matching symbol in ${path.relative(root, source)}`);
      }
    }
  }

  if (problems.length) {
    console.error(`docs-hover: ${problems.length} issue(s):\n` + problems.map((p) => "  - " + p).join("\n"));
    process.exit(1);
  }
  console.log("docs-hover: all doc sections resolve to symbols");
}

main();
