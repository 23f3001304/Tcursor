// Guards for the editor's two-theme palette (see editor.css's "THE TWO PALETTE BLOCKS" header).
//
// 1. Parity: every `--e-*` the dark block declares is declared by BOTH light blocks, and the two
//    light blocks are value-for-value identical - so a token can never be added dark-only (the
//    editor would keep a dark chip in a light window) and the explicit `data-theme="light"` copy
//    can never drift from the `prefers-color-scheme` one that serves the "system" setting.
// 2. No raw hex anywhere in the editor's stylesheets outside those blocks, so a colour can only
//    ever enter the app through a token that both themes answered for.
//
// Two forms are deliberately NOT violations of rule 2, and neither needs an allow-list entry:
//   - comments (stripped before scanning - the sheets explain their palette in prose);
//   - a `var(--token, #hex)` fallback. That is still token-FIRST: the hex only paints where the
//     token is undefined, which for src/lib/wave.css means the HUD's own surfaces, a sheet this
//     task does not own. Those are stripped structurally rather than listed one by one.
// ALLOWED below is the escape hatch for a genuine literal. It is empty: every colour that assumed
// a dark ground now reads a token. The literals that legitimately stay (the modal scrim's black,
// the black separator rings and white inner hairline on the handles drawn OVER the composited
// video, `.e-ahint`'s dark pill) are all rgba(), not hex, and are argued for in their own comments
// where they are written. The canvas draws that keep fixed colours - previewCanvas.ts's panel
// shadow, black backing and white PiP edge, stageBg.ts's placeholder gradient - are the EXPORT's
// own pixels rather than chrome, live in .ts rather than CSS, and are outside this scan.
import { describe, it, expect } from "vitest";

// The sheets are read off disk rather than imported: vitest stubs every `.css` import to "" (its
// `test.css` option is off), `?raw` included. `node:fs` comes in through a non-literal specifier
// and `process` off `globalThis` because the app ships no @types/node and a literal import of
// either would fail `tsc --noEmit`; both resolve normally at run time, where vitest is Node.
const NODE_FS = "node:fs";
const fs = (await import(/* @vite-ignore */ NODE_FS)) as {
  readFileSync(p: string, enc: "utf8"): string;
  readdirSync(p: string, o: { withFileTypes: true }): { name: string; isDirectory(): boolean }[];
};
const node = globalThis as unknown as { process: { cwd(): string } };
const SRC = `${node.process.cwd().replace(/\\/g, "/")}/src`;
const read = (rel: string) => fs.readFileSync(`${SRC}/${rel}`, "utf8");

/** Every stylesheet the editor paints from, paths under `src`. `lib/wave.css` is in the list
 *  because the wave components mount inside the editor and read its tokens. */
const SHEETS = [
  "editor/editor.css",
  "editor/controls/controls.css",
  "editor/effects/effects.css",
  "editor/inspectors/inspectors.css",
  "editor/panels/panels.css",
  "editor/stage/stage.css",
  "editor/timeline/timeline.css",
  "lib/wave.css",
];

/** The three palette blocks, by the selector that opens each. Dark first - it is the default. */
const PALETTE_SELECTORS = [
  ".editor {",
  ':root[data-theme="light"] .editor {',
  ':root:not([data-theme="dark"]) .editor {',
] as const;

/** Literal colours that may stay raw, keyed `<sheet>:<hex>`, value = why. Empty on purpose. */
const ALLOWED: Record<string, string> = {};

/** Replace `[from, to)` with spaces, keeping every newline, so line numbers survive stripping. */
function blank(text: string, from: number, to: number): string {
  return text.slice(0, from) + text.slice(from, to).replace(/[^\n]/g, " ") + text.slice(to);
}

function stripComments(css: string): string {
  let out = css;
  for (;;) {
    const start = out.indexOf("/*");
    if (start < 0) return out;
    const end = out.indexOf("*/", start + 2);
    out = blank(out, start, end < 0 ? out.length : end + 2);
  }
}

/** Blank the fallback of every `var(--x, <fallback>)` (one level of nesting is enough here). */
function stripVarFallbacks(css: string): string {
  return css.replace(/var\(\s*--[\w-]+\s*,(?:[^()]|\([^()]*\))*\)/g, (m) => m.replace(/[^\n]/g, " "));
}

/** The brace-balanced block whose opening `selector {` comes first in `css`. */
function blockAt(css: string, selector: string): { body: string; start: number; end: number } {
  const open = css.indexOf(selector);
  if (open < 0) throw new Error(`editor.css has no \`${selector}\` block`);
  let depth = 0;
  for (let i = open + selector.length - 1; i < css.length; i++) {
    if (css[i] === "{") depth++;
    else if (css[i] === "}" && --depth === 0) {
      return { body: css.slice(open + selector.length, i), start: open, end: i + 1 };
    }
  }
  throw new Error(`unterminated \`${selector}\` block in editor.css`);
}

/** The `--e-name: value` declarations of one block body. */
function tokensOf(body: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const m of body.matchAll(/(--e-[\w-]+)\s*:\s*([^;}]+)/g)) {
    out.set(m[1], m[2].trim().replace(/\s+/g, " "));
  }
  return out;
}

/** Every `.css` under `src/editor`, so a new sheet cannot quietly dodge the scan. */
function editorSheetsOnDisk(dir = "editor"): string[] {
  const out: string[] = [];
  for (const e of fs.readdirSync(`${SRC}/${dir}`, { withFileTypes: true })) {
    if (e.isDirectory()) out.push(...editorSheetsOnDisk(`${dir}/${e.name}`));
    else if (e.name.endsWith(".css")) out.push(`${dir}/${e.name}`);
  }
  return out;
}

describe("editor palette: the light theme answers for every token", () => {
  const css = stripComments(read("editor/editor.css"));
  const dark = blockAt(css, PALETTE_SELECTORS[0]);
  const light = blockAt(css, PALETTE_SELECTORS[1]);
  const system = blockAt(css, PALETTE_SELECTORS[2]);

  it("declares the same `--e-*` names in the dark block and in both light blocks", () => {
    const darkNames = [...tokensOf(dark.body).keys()].sort();
    expect(darkNames.length).toBeGreaterThan(20); // a real block, not an empty accidental match
    expect([...tokensOf(light.body).keys()].sort()).toEqual(darkNames);
    expect([...tokensOf(system.body).keys()].sort()).toEqual(darkNames);
  });

  it("keeps the data-theme block and the prefers-color-scheme block value-identical", () => {
    expect(Object.fromEntries(tokensOf(system.body))).toEqual(Object.fromEntries(tokensOf(light.body)));
  });

  it("moves every palette token between the themes except the two that cannot", () => {
    // The ones that are the SAME in both themes are exactly those whose ground is not the theme:
    // white on a saturated fill or a caption scrim, and the constant cursor plate. The dim rung
    // used to be shared too (#71717a from either side) until the owner found the light theme
    // too pale to read (2026-09-14); it is now a darker grey there, like every other text rung.
    const lightTokens = tokensOf(light.body);
    const same = [...tokensOf(dark.body)].filter(([k, v]) => lightTokens.get(k) === v).map(([k]) => k);
    expect(same.sort()).toEqual(["--e-on-fill", "--e-plate"]);
  });

  it("leaves the dark palette as the default, so an editor with no theme attribute is unchanged", () => {
    expect(dark.start).toBeLessThan(light.start);
    expect(css.slice(0, dark.start)).not.toContain("data-theme"); // no guard above the dark block
  });
});

describe("editor stylesheets: colour only ever enters through a token", () => {
  it("scans every stylesheet that exists under src/editor", () => {
    expect(editorSheetsOnDisk().sort()).toEqual(SHEETS.filter((s) => s.startsWith("editor/")).sort());
  });

  it.each(SHEETS)("%s has no raw hex colour outside the palette blocks", (sheet) => {
    let css = stripVarFallbacks(stripComments(read(sheet)));
    if (sheet === "editor/editor.css") {
      // Blank the palette blocks themselves - they are the one place a literal belongs. `blank`
      // keeps the string's length, so all three spans stay valid while they are applied.
      for (const span of PALETTE_SELECTORS.map((s) => blockAt(css, s))) css = blank(css, span.start, span.end);
    }
    const offenders: string[] = [];
    for (const m of css.matchAll(/#[0-9a-fA-F]{3,8}\b/g)) {
      if (ALLOWED[`${sheet}:${m[0].toLowerCase()}`]) continue;
      offenders.push(`line ${css.slice(0, m.index).split("\n").length}: ${m[0]}`);
    }
    expect(offenders).toEqual([]);
  });
});
