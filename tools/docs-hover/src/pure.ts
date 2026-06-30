// Pure helpers with no `vscode` dependency, so they run under plain Node for
// the validator and unit tests. The extension host code lives in docstore.ts /
// extension.ts and reuses everything here.
import * as path from "path";

/** A parsed doc file: section name (`## heading`) -> trimmed Markdown body. */
export type DocMap = Map<string, string>;

/** Split a Markdown doc into its `## heading` -> body sections (ignores `#`/`###`). */
export function parseDoc(text: string): DocMap {
  const map: DocMap = new Map();
  let heading: string | null = null;
  let buf: string[] = [];
  const flush = () => { if (heading !== null) map.set(heading, buf.join("\n").trim()); };
  for (const line of text.split(/\r?\n/)) {
    const m = /^##\s+(.+?)\s*$/.exec(line);
    if (m && !line.startsWith("###")) { flush(); heading = m[1].trim(); buf = []; }
    else if (heading !== null) { buf.push(line); }
  }
  flush();
  return map;
}

/**
 * Body for `name`. An exact `## name` section wins; otherwise the first
 * `## Type::name` section in file order matches (the receiver type is not resolved).
 */
export function sectionFor(map: DocMap, name: string): string | undefined {
  const exact = map.get(name);
  if (exact !== undefined) return exact;
  for (const [key, body] of map) if (key.endsWith("::" + name)) return body;
  return undefined;
}

/** The docs/api Markdown file that mirrors a source file (extension swapped to .md). */
export function docPathForSource(workspaceRoot: string, sourceFsPath: string): string {
  const rel = path.relative(workspaceRoot, sourceFsPath).replace(/\.[^./\\]+$/, "");
  return path.join(workspaceRoot, "docs", "api", rel + ".md");
}

/** Source files a docs/api Markdown file could document (.rs / .ts / .tsx). */
export function sourceCandidatesForDoc(workspaceRoot: string, docFsPath: string): string[] {
  const apiRoot = path.join(workspaceRoot, "docs", "api");
  const rel = path.relative(apiRoot, docFsPath).replace(/\.md$/, "");
  return [".rs", ".ts", ".tsx"].map((ext) => path.join(workspaceRoot, rel + ext));
}

/**
 * Heuristic (not a parser): does `source` declare a symbol named `name`? Matches
 * a declaration keyword + name at a declaration boundary - line start, or right
 * after an attribute `]`, an inline block-open `{` (e.g. `impl T { pub fn x }`),
 * or `;` - tolerating pub/export/default/async/const/unsafe modifiers. This
 * handles the codebase's one-line `#[derive(..)] pub struct X` and inline impls.
 * It does not do scope analysis, so a same-named local could still satisfy a
 * stale doc section; the drift guard is a safety net, not a proof.
 */
export function definesSymbol(source: string, name: string): boolean {
  const n = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const boundary = `(^|[\\]{;])\\s*`;
  const vis = `(pub(\\([^)]*\\))?\\s+|export\\s+|default\\s+|async\\s+)*`;
  const kw = `((const\\s+|async\\s+|unsafe\\s+|extern\\s+("[^"]*"\\s+)?)*fn|function|struct|enum|trait|type|class|interface|const|static|mod|namespace)`;
  const decl = new RegExp(`${boundary}${vis}${kw}\\s+${n}\\b`, "m");
  return decl.test(source);
}
