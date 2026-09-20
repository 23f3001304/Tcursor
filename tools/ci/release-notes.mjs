// Writes the description of a GitHub release: what TCursor is, what changed (the tagged
// version's section of CHANGELOG.md), how to install, and what the files hash to.
// A release candidate (v0.1.0-rc.1) uses the section of the version it is a candidate for.
// A version without a section is refused, so a release cannot go out without saying what changed.
// Usage: node tools/ci/release-notes.mjs <tag> <SHA256SUMS file> <output file>
//        node tools/ci/release-notes.mjs <tag> --check      only checks the tag and its section,
//                                                           so a release fails before the long build
//   reads CHANGELOG.md from the working directory; GITHUB_REPOSITORY, GITHUB_SHA,
//   GITHUB_SERVER_URL and GITHUB_RUN_ID name the build, SIGNED=true marks a code-signed one.
//   With GITHUB_OUTPUT set it also reports title=TCursor <version> for the release's name.
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export function versionOf(tag) {
  const m = /^v(\d+\.\d+\.\d+)(?:-([0-9A-Za-z.-]+))?$/.exec(tag);
  if (!m) throw new Error(`the tag "${tag}" is not v<major>.<minor>.<patch> or v<major>.<minor>.<patch>-<pre>`);
  return { version: m[1], pre: m[2] ?? "" };
}

export function sectionOf(changelog, version) {
  const lines = changelog.replace(/\r\n/g, "\n").split("\n");
  const escaped = version.replace(/\./g, "\\.");
  const heading = new RegExp(`^##\\s+\\[?${escaped}\\]?(\\s|$)`);
  const start = lines.findIndex((l) => heading.test(l));
  if (start < 0) throw new Error(`CHANGELOG.md has no "## ${version}" section: write what changed before tagging`);
  const rest = lines.slice(start + 1);
  const next = rest.findIndex((l) => /^##(?!#)/.test(l));
  const section = (next < 0 ? rest : rest.slice(0, next)).join("\n").trim();
  if (!section) throw new Error(`the CHANGELOG.md section for ${version} is empty`);
  return section;
}

const UNSIGNED =
  '> The installers are not code-signed yet, so Windows SmartScreen shows "Windows protected your PC". ' +
  "Click **More info**, then **Run anyway**. You can check a download against the checksums below, " +
  "and you can build from source, which never triggers SmartScreen.";
const SIGNED = "> These installers are code-signed. Free code signing provided by SignPath.io, certificate by SignPath Foundation.";

export function compose({ tag, section, sums, repo, sha, runUrl, signed }) {
  const { version, pre } = versionOf(tag);
  const out = [
    "TCursor is an auto-zoom screen recorder for Windows that turns a raw capture into a polished demo: " +
      "automatic cursor-follow zooms, a built-in editor, on-device captions, and a local AI director that suggests edits.",
    "",
  ];
  if (pre) {
    out.push(`> This is a pre-release (${pre}) of ${version}: a build to try, not the final ${version}.`, "");
  }
  out.push("## What changed", "", section, "");
  out.push(
    "## Install",
    "",
    "64-bit Windows 10 or 11. Download **`TCursorSetup.exe`** below and run it: it is the branded installer and " +
      "the one most people want. The plain NSIS installer (`-setup.exe`) and the MSI install the same app, for " +
      "scripted or managed installs.",
    "",
    signed ? SIGNED : UNSIGNED,
    "",
    "TCursor needs a graphics driver that provides Vulkan, which every current NVIDIA, AMD and Intel driver does. " +
      "It has no account, no telemetry and no update check; the README's Privacy section says exactly when it uses the network.",
    "",
    "## Checksums (SHA-256)",
    "",
    "```",
    sums.replace(/\r\n/g, "\n").trim(),
    "```",
    "",
    `Built from commit [${sha.slice(0, 8)}](https://github.com/${repo}/commit/${sha}) by [this workflow run](${runUrl}) ` +
      "on a clean GitHub runner. Nobody uploads a locally built binary.",
    "",
  );
  return out.join("\n");
}

export function titleOf(tag) {
  const { version, pre } = versionOf(tag);
  return `TCursor ${version}${pre ? `-${pre}` : ""}`;
}

function main() {
  const [tag, sumsFile, outFile] = process.argv.slice(2);
  const env = process.env;
  const section = sectionOf(readFileSync("CHANGELOG.md", "utf8"), versionOf(tag ?? "").version);
  if (sumsFile === "--check") {
    console.log(`${titleOf(tag)}: CHANGELOG.md has a section of ${section.split("\n").length} lines`);
    return;
  }
  if (!sumsFile || !outFile) throw new Error("usage: release-notes.mjs <tag> <SHA256SUMS file> <output file>");
  if (env.GITHUB_OUTPUT) appendFileSync(env.GITHUB_OUTPUT, `title=${titleOf(tag)}\n`);
  const repo = env.GITHUB_REPOSITORY ?? "";
  const body = compose({
    tag,
    section,
    sums: readFileSync(sumsFile, "utf8"),
    repo,
    sha: env.GITHUB_SHA ?? "",
    runUrl: `${env.GITHUB_SERVER_URL ?? "https://github.com"}/${repo}/actions/runs/${env.GITHUB_RUN_ID ?? ""}`,
    signed: env.SIGNED === "true",
  });
  writeFileSync(outFile, body);
  console.log(`release notes for ${tag}: ${body.split("\n").length} lines in ${outFile}`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    main();
  } catch (e) {
    console.log(`::error title=Release notes::${e.message}`);
    process.exit(1);
  }
}
