import { test } from "node:test";
import assert from "node:assert/strict";
import { compose, sectionOf, titleOf, versionOf } from "./release-notes.mjs";

const CHANGELOG = [
  "# Changelog",
  "",
  "Intro text.",
  "",
  "## 0.2.0 - 2026-10-01",
  "",
  "### Editor",
  "- A second thing.",
  "",
  "## [0.1.0]",
  "",
  "First public release.",
  "",
  "### Recording",
  "- Records the screen.",
  "",
  "## 0.0.9",
  "",
  "- Older.",
  "",
].join("\n");

test("a tag names a version, and a dash marks a pre-release", () => {
  assert.deepEqual(versionOf("v0.1.0"), { version: "0.1.0", pre: "" });
  assert.deepEqual(versionOf("v0.1.0-rc.1"), { version: "0.1.0", pre: "rc.1" });
  assert.deepEqual(versionOf("v12.30.4-beta.2"), { version: "12.30.4", pre: "beta.2" });
});

test("the release is named after the product and the version, without the tag's v", () => {
  assert.equal(titleOf("v0.1.0"), "TCursor 0.1.0");
  assert.equal(titleOf("v0.1.0-rc.1"), "TCursor 0.1.0-rc.1");
});

test("anything that is not v<major>.<minor>.<patch> is refused", () => {
  for (const tag of ["main", "0.1.0", "v0.1", "v0.1.0rc1", ""]) assert.throws(() => versionOf(tag), /tag/);
});

test("the section of a version runs to the next version heading and keeps its subsections", () => {
  const section = sectionOf(CHANGELOG, "0.1.0");
  assert.equal(section, ["First public release.", "", "### Recording", "- Records the screen."].join("\n"));
  assert.equal(sectionOf(CHANGELOG, "0.2.0"), ["### Editor", "- A second thing."].join("\n"));
  assert.equal(sectionOf(CHANGELOG, "0.0.9"), "- Older.");
});

test("a version is matched whole, so 0.1.0 never answers for 0.1.0x or 10.1.0", () => {
  const log = "## 10.1.0\n\n- ten\n\n## 0.1.01\n\n- padded\n";
  assert.throws(() => sectionOf(log, "0.1.0"), /CHANGELOG/);
});

test("a release without a changelog section, or with an empty one, is refused", () => {
  assert.throws(() => sectionOf(CHANGELOG, "0.3.0"), /0\.3\.0/);
  assert.throws(() => sectionOf("## 0.3.0\n\n## 0.2.0\n\n- x\n", "0.3.0"), /empty/);
});

const base = {
  tag: "v0.1.0",
  section: "### Recording\n- Records the screen.",
  sums: "aaaa  TCursorSetup.exe\nbbbb  TCursor_0.1.0_x64-setup.exe\n",
  repo: "someone/tcursor",
  sha: "0123456789abcdef0123456789abcdef01234567",
  runUrl: "https://github.com/someone/tcursor/actions/runs/1",
  signed: false,
};

test("the body says what changed, how to install, and what the files hash to", () => {
  const body = compose(base);
  assert.match(body, /## What changed\n\n### Recording\n- Records the screen\./);
  assert.match(body, /## Install/);
  assert.match(body, /TCursorSetup\.exe/);
  assert.match(body, /## Checksums \(SHA-256\)\n\n```\naaaa  TCursorSetup\.exe\nbbbb  TCursor_0\.1\.0_x64-setup\.exe\n```/);
  assert.match(body, /someone\/tcursor\/commit\/0123456789abcdef0123456789abcdef01234567/);
  assert.match(body, /actions\/runs\/1/);
});

test("only a pre-release tag is announced as one", () => {
  assert.doesNotMatch(compose(base), /release candidate|pre-release/i);
  assert.match(compose({ ...base, tag: "v0.1.0-rc.1" }), /pre-release \(rc\.1\) of 0\.1\.0/);
});

test("an unsigned build explains SmartScreen, a signed one credits SignPath instead", () => {
  assert.match(compose(base), /SmartScreen/);
  assert.doesNotMatch(compose(base), /SignPath Foundation/);
  const signed = compose({ ...base, signed: true });
  assert.doesNotMatch(signed, /SmartScreen/);
  assert.match(signed, /Free code signing provided by SignPath\.io, certificate by SignPath Foundation/);
});
