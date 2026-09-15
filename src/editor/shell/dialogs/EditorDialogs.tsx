import type { EditDoc } from "../../../shared/edit";
import { exportProject } from "../../../shared/ipc";
import { ExportDialog } from "./ExportDialog";
import { ShortcutsOverlay } from "./ShortcutsOverlay";
import { EditorSettingsDialog } from "../settings/EditorSettingsDialog";
import type { useExportState } from "../../hooks/doc/useExportState";

export function EditorDialogs({
  folder,
  settings,
  exportState,
  showExport,
  onCloseExport,
  showShortcuts,
  onCloseShortcuts,
  showSettings,
  onCloseSettings,
  onOpenShortcuts,
  onSaveSettings,
  onApplyMotion,
}: {
  folder: string;
  settings: EditDoc["settings"];
  exportState: ReturnType<typeof useExportState>;
  showExport: boolean;
  onCloseExport: () => void;
  showShortcuts: boolean;
  onCloseShortcuts: () => void;
  showSettings: boolean;
  onCloseSettings: () => void;
  onOpenShortcuts: () => void;
  onSaveSettings: (s: EditDoc["settings"]) => void;
  onApplyMotion: () => void;
}) {
  const x = exportState;
  const reset = () => {
    x.setExportDone(false);
    x.setExportError(null);
    x.setExportPath("");
  };
  return (
    <>
      <ExportDialog
        open={showExport}
        exporting={x.exporting}
        pct={x.pct}
        done={x.exportDone}
        error={x.exportError}
        exportPath={x.exportPath}
        startedAt={x.exportStartedAt}
        onClose={onCloseExport}
        onReset={reset}
        onExport={(s) => {
          reset();
          x.startExport();
          void exportProject(folder, s);
        }}
      />
      <ShortcutsOverlay open={showShortcuts} onClose={onCloseShortcuts} />
      <EditorSettingsDialog
        open={showSettings}
        settings={settings}
        onClose={onCloseSettings}
        onSaveSettings={onSaveSettings}
        onOpenShortcuts={onOpenShortcuts}
        onApplyToAll={onApplyMotion}
      />
    </>
  );
}
