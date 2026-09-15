import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

export function useExportState(onExportWarning?: (msg: string) => void) {
  const [exporting, setExporting] = useState(false);
  const [pct, setPct] = useState(0);
  const [exportDone, setExportDone] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
  const [exportPath, setExportPath] = useState("");
  const exportStartRef = useRef<number | null>(null);
  const startExport = useCallback(() => {
    setExporting(true);
    setPct(0);
    exportStartRef.current = Date.now();
  }, []);

  const onExportWarningRef = useRef(onExportWarning);
  onExportWarningRef.current = onExportWarning;
  useEffect(() => {
    const subs = [
      listen<number>("export-progress", (e) => setPct(e.payload)),
      listen<string>("export-done", (e) => {
        setExporting(false);
        setExportDone(true);
        setExportPath(e.payload);
      }),
      listen<string>("export-error", (e) => {
        setExporting(false);
        setExportError(e.payload);
      }),
      listen<string>("export-warning", (e) => onExportWarningRef.current?.(e.payload)),
    ];
    return () => {
      subs.forEach((s) => s.then((f) => f()));
    };
  }, []);

  return {
    exporting,
    setExporting,
    pct,
    setPct,
    exportDone,
    setExportDone,
    exportError,
    setExportError,
    exportPath,
    setExportPath,
    startExport,
    exportStartedAt: exportStartRef.current,
  };
}
