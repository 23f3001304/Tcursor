// @vitest-environment jsdom

import { describe, it, expect } from "vitest";

const NODE_FS = "node:fs";
const fs = (await import(NODE_FS)) as {
  readFileSync(p: string, enc: "utf8"): string;
  readdirSync(p: string, o: { withFileTypes: true }): { name: string; isDirectory(): boolean }[];
};
const node = globalThis as unknown as { process: { cwd(): string } };
const SRC = `${node.process.cwd().replace(/\\/g, "/")}/src`;
const read = (rel: string) => fs.readFileSync(`${SRC}/${rel}`, "utf8");

const SHEETS = [
  "editor/editor.css",
  "editor/controls/controls.css",
  "editor/effects/effects.css",
  "editor/inspectors/inspectors.css",
  "editor/motion/motion.css",
  "editor/panels/panels.css",
  "editor/stage/stage.css",
  "editor/timeline/timeline.css",
  "shared/wave.css",
];

const PALETTE_SELECTORS = [
  ".editor {",
  ':root[data-theme="light"] .editor {',
  ':root:not([data-theme="dark"]) .editor {',
] as const;

const ALLOWED: Record<string, string> = {};

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

function stripVarFallbacks(css: string): string {
  return css.replace(/var\(\s*--[\w-]+\s*,(?:[^()]|\([^()]*\))*\)/g, (m) => m.replace(/[^\n]/g, " "));
}

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

function tokensOf(body: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const m of body.matchAll(/(--e-[\w-]+)\s*:\s*([^;}]+)/g)) {
    out.set(m[1], m[2].trim().replace(/\s+/g, " "));
  }
  return out;
}

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
    expect(darkNames.length).toBeGreaterThan(20);
    expect([...tokensOf(light.body).keys()].sort()).toEqual(darkNames);
    expect([...tokensOf(system.body).keys()].sort()).toEqual(darkNames);
  });

  it("keeps the data-theme block and the prefers-color-scheme block value-identical", () => {
    expect(Object.fromEntries(tokensOf(system.body))).toEqual(Object.fromEntries(tokensOf(light.body)));
  });

  it("moves every palette token between the themes except the two that cannot", () => {
    const lightTokens = tokensOf(light.body);
    const same = [...tokensOf(dark.body)].filter(([k, v]) => lightTokens.get(k) === v).map(([k]) => k);
    expect(same.sort()).toEqual(["--e-on-fill", "--e-plate"]);
  });

  it("leaves the dark palette as the default, so an editor with no theme attribute is unchanged", () => {
    expect(dark.start).toBeLessThan(light.start);
    expect(css.slice(0, dark.start)).not.toContain("data-theme");
  });
});

describe("editor stylesheets: colour only ever enters through a token", () => {
  it("scans every stylesheet that exists under src/editor", () => {
    expect(editorSheetsOnDisk().sort()).toEqual(SHEETS.filter((s) => s.startsWith("editor/")).sort());
  });

  it.each(SHEETS)("%s has no raw hex colour outside the palette blocks", (sheet) => {
    let css = stripVarFallbacks(stripComments(read(sheet)));
    if (sheet === "editor/editor.css") {
      for (const span of PALETTE_SELECTORS.map((s) => blockAt(css, s)))
        css = blank(css, span.start, span.end);
    }
    const offenders: string[] = [];
    for (const m of css.matchAll(/#[0-9a-fA-F]{3,8}\b/g)) {
      if (ALLOWED[`${sheet}:${m[0].toLowerCase()}`]) continue;
      offenders.push(`line ${css.slice(0, m.index).split("\n").length}: ${m[0]}`);
    }
    expect(offenders).toEqual([]);
  });
});
