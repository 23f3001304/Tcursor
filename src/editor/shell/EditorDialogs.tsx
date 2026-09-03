import type { EditDoc } from "../../lib/edit";
import { exportProject } from "../../lib/ipc";
import { ExportDialog } from "./ExportDialog";
import { ShortcutsOverlay } from "./ShortcutsOverlay";
import { EditorSettingsDialog } from "./settings/EditorSettingsDialog";
import type { useExportState } from "../hooks/useExportState";

/** The editor's modal stack - Export, Shortcuts and Settings - lifted out of `Editor.tsx` as one
 *  unit so that file stays under its line budget. It owns no state of its own: every flag and
 *  setter still lives in `Editor`, which is also what `modalOpen` (the keymap's inert-behind-a-
 *  modal gate) is computed from. The export kickoff lives here because it is exactly the three
 *  result-state resets plus the IPC call, and nothing else reads them. */
export function EditorDialogs({ folder, settings, exportState, showExport, onCloseExport, showShortcuts, onCloseShortcuts, showSettings, onCloseSettings, onOpenShortcuts, onSaveSettings }: {
  folder: string;
  settings: EditDoc["settings"];
  exportState: ReturnType<typeof useExportState>;
  showExport: boolean; onCloseExport: () => void;
  showShortcuts: boolean; onCloseShortcuts: () => void;
  showSettings: boolean; onCloseSettings: () => void;
  onOpenShortcuts: () => void;
  onSaveSettings: (s: EditDoc["settings"]) => void;
}) {
  const x = exportState;
  const reset = () => { x.setExportDone(false); x.setExportError(null); x.setExportPath(""); };
  return (
    <>
      <ExportDialog open={showExport} exporting={x.exporting} pct={x.pct} done={x.exportDone} error={x.exportError}
        exportPath={x.exportPath} startedAt={x.exportStartedAt} onClose={onCloseExport} onReset={reset}
        onExport={(s) => { reset(); x.startExport(); void exportProject(folder, s); }} />
      <ShortcutsOverlay open={showShortcuts} onClose={onCloseShortcuts} />
      <EditorSettingsDialog open={showSettings} settings={settings} onClose={onCloseSettings}
        onSaveSettings={onSaveSettings} onOpenShortcuts={onOpenShortcuts} />
    </>
  );
}
