import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

/** Export-run state (progress/done/error/path) plus the export-* IPC listeners - split out of
 *  useEditorData.ts (already at its own line budget) so this concern has its own home.
 *  `startExport`/`exportStartedAt` exist so the ETA baseline (ExportProgress) survives the export
 *  dialog being closed and reopened mid-export - a supported flow - instead of resetting back to
 *  "just started" on remount (D Low). `onExportWarning` wires Rust's `export-warning` event
 *  (webcam missing/frozen in an otherwise-successful export - Task 9 added the emit, nothing ever
 *  listened) through to the caller; Editor.tsx passes the Toast pill's `push`. */
export function useExportState(onExportWarning?: (msg: string) => void) {
  const [exporting, setExporting] = useState(false);
  const [pct, setPct] = useState(0);
  const [exportDone, setExportDone] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
  const [exportPath, setExportPath] = useState(""); // export-done's payload (an absolute path), for "Show in folder"
  const exportStartRef = useRef<number | null>(null);
  const startExport = useCallback(() => { setExporting(true); setPct(0); exportStartRef.current = Date.now(); }, []);

  // Ref, not a dep, so the listener effect below keeps its empty `[]` deps and never re-subscribes.
  const onExportWarningRef = useRef(onExportWarning); onExportWarningRef.current = onExportWarning;
  useEffect(() => {
    const subs = [
      listen<number>("export-progress", (e) => setPct(e.payload)),
      listen<string>("export-done", (e) => { setExporting(false); setExportDone(true); setExportPath(e.payload); }),
      listen<string>("export-error", (e) => { setExporting(false); setExportError(e.payload); }),
      // Non-fatal, emitted BEFORE export-done, never instead of it - doesn't touch exporting/done.
      listen<string>("export-warning", (e) => onExportWarningRef.current?.(e.payload)),
    ];
    return () => { subs.forEach((s) => s.then((f) => f())); };
  }, []);

  return {
    exporting, setExporting, pct, setPct, exportDone, setExportDone, exportError, setExportError,
    exportPath, setExportPath, startExport, exportStartedAt: exportStartRef.current,
  };
}
