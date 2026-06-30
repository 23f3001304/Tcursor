// Plain-Node unit tests for the pure helpers (no vscode host needed).
// Run: `npm test` (compiles then `node out/pure.test.js`).
import * as path from "path";
import { definesSymbol, docPathForSource, parseDoc, sectionFor, sourceCandidatesForDoc } from "./pure";

let count = 0;
const ok = (cond: boolean, msg: string) => {
  count++;
  if (!cond) { console.error("FAIL: " + msg); process.exitCode = 1; }
};

const doc = "# Title\n\n## apply\nApplies an op.\n\nMore detail.\n\n## metrics\nComputes metrics.\n";
const m = parseDoc(doc);
ok(m.get("apply") === "Applies an op.\n\nMore detail.", "parse: apply body");
ok(m.get("metrics") === "Computes metrics.", "parse: metrics body");
ok(m.size === 2, "parse: ignores level-1 title");
ok(parseDoc("## a\n### sub\nbody").get("a") === "### sub\nbody", "parse: ignores level-3");

ok(sectionFor(m, "apply") !== undefined, "sectionFor: exact match");
ok(sectionFor(parseDoc("## EditDoc::load\nLoads."), "load") === "Loads.", "sectionFor: ::method suffix");
ok(sectionFor(m, "missing") === undefined, "sectionFor: miss returns undefined");

const dp = docPathForSource("/repo", path.join("/repo", "src-tauri", "src", "edit", "api.rs"));
ok(dp === path.join("/repo", "docs", "api", "src-tauri", "src", "edit", "api.md"), "docPathForSource: mirror tree");

const cands = sourceCandidatesForDoc("/repo", path.join("/repo", "docs", "api", "src", "lib", "edit.md"));
ok(cands.some((c) => c.endsWith("edit.ts")) && cands.some((c) => c.endsWith("edit.rs")), "candidates: rs+ts");

ok(definesSymbol("pub fn apply(doc: &mut EditDoc) {}", "apply"), "defines: rust fn");
ok(definesSymbol("export function ops_from_json() {}", "ops_from_json"), "defines: ts function");
ok(definesSymbol("export const getEdit = (f: string) => 1;", "getEdit"), "defines: ts const arrow");
ok(definesSymbol("export interface EditDoc { version: number }", "EditDoc"), "defines: ts interface");
ok(definesSymbol("pub(crate) fn helper() {}", "helper"), "defines: pub(crate) fn");
ok(definesSymbol("export async function load() {}", "load"), "defines: export async function");
ok(definesSymbol("export enum Mode { A }", "Mode"), "defines: ts enum");
ok(definesSymbol("#[derive(Clone, Copy)] pub struct Rgb { pub r: u8 }", "Rgb"), "defines: inline attr + struct");
ok(definesSymbol("impl FrameIndex { pub fn next(self) -> FrameIndex {} }", "next"), "defines: inline impl method");
ok(definesSymbol("pub const fn zero() -> u8 { 0 }", "zero"), "defines: const fn");
ok(definesSymbol(`unsafe extern "system" fn hook_proc() {}`, "hook_proc"), "defines: unsafe extern fn");
ok(!definesSymbol("  return applyThing(x);", "applyThing"), "defines: mid-line use is not a decl");
ok(!definesSymbol("let other = 1;", "apply"), "defines: absent symbol is false");

console.log(`pure.test: ${count} assertions, ${process.exitCode ? "FAILED" : "passed"}`);
