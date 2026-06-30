import * as vscode from "vscode";
import { DocMap, docPathForSource, parseDoc, sectionFor } from "./pure";

/**
 * Loads and caches the docs/api Markdown store, resolving the doc section for a
 * symbol given the file where it is defined. A null cache entry means "no doc
 * file for that source", so undocumented files cost one failed read, then zero.
 */
export class DocStore {
  private cache = new Map<string, DocMap | null>();

  constructor(private readonly root: string) {}

  /** Markdown for `name` as defined in `sourceUri`, or undefined if undocumented. */
  async lookup(sourceUri: vscode.Uri, name: string): Promise<vscode.MarkdownString | undefined> {
    const map = await this.load(docPathForSource(this.root, sourceUri.fsPath));
    if (!map) return undefined;
    const body = sectionFor(map, name);
    if (!body) return undefined;
    const md = new vscode.MarkdownString(body);
    md.isTrusted = false;
    return md;
  }

  /** Forget a cached doc file so the next lookup re-reads it (on fs changes). */
  invalidate(docFsPath: string): void { this.cache.delete(docFsPath); }

  private async load(docPath: string): Promise<DocMap | null> {
    const hit = this.cache.get(docPath);
    if (hit !== undefined) return hit;
    let parsed: DocMap | null = null;
    try {
      const bytes = await vscode.workspace.fs.readFile(vscode.Uri.file(docPath));
      parsed = parseDoc(Buffer.from(bytes).toString("utf8"));
    } catch {
      parsed = null;
    }
    this.cache.set(docPath, parsed);
    return parsed;
  }
}
