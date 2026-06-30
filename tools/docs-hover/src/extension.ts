import * as vscode from "vscode";
import { DocStore } from "./docstore";

const LANGS = ["rust", "typescript", "typescriptreact"];

/** Wire the hover provider for Rust/TS and a watcher that drops stale doc caches. */
export function activate(context: vscode.ExtensionContext): void {
  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) return;
  const store = new DocStore(folder.uri.fsPath);

  const provider: vscode.HoverProvider = {
    async provideHover(document, position, token) {
      const range = document.getWordRangeAtPosition(position);
      if (!range) return undefined;
      const name = document.getText(range);
      const owner = (await resolveDefUri(document, position)) ?? document.uri;
      if (token.isCancellationRequested) return undefined;
      const md = await store.lookup(owner, name);
      return md ? new vscode.Hover(md, range) : undefined;
    },
  };
  for (const lang of LANGS) {
    context.subscriptions.push(vscode.languages.registerHoverProvider(lang, provider));
  }

  const watcher = vscode.workspace.createFileSystemWatcher(
    new vscode.RelativePattern(folder, "docs/api/**/*.md")
  );
  const drop = (uri: vscode.Uri) => store.invalidate(uri.fsPath);
  watcher.onDidChange(drop);
  watcher.onDidCreate(drop);
  watcher.onDidDelete(drop);
  context.subscriptions.push(watcher);
}

export function deactivate(): void {}

/** Ask the language server which file defines the hovered symbol (disambiguates same names). */
async function resolveDefUri(
  document: vscode.TextDocument,
  position: vscode.Position
): Promise<vscode.Uri | undefined> {
  try {
    const defs = await vscode.commands.executeCommand<(vscode.Location | vscode.LocationLink)[]>(
      "vscode.executeDefinitionProvider", document.uri, position
    );
    const first = defs?.[0];
    if (!first) return undefined;
    return (first as vscode.LocationLink).targetUri ?? (first as vscode.Location).uri;
  } catch {
    return undefined;
  }
}
