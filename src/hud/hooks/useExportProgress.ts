import { useEffect, useState, type RefObject } from "react";
import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

export function useExportProgress(lastFolderRef: RefObject<string>) {
  const [exporting, setExporting] = useState(false);
  const [pct, setPct] = useState(0);
  const [exportErr, setExportErr] = useState<string | null>(null);

  useEffect(() => {
    const unsubs: Promise<() => void>[] = [];
    unsubs.push(
      listen<number>("export-progress", (e) => {
        setPct(e.payload);
        setExportErr(null);
      }),
    );
    unsubs.push(
      listen<string>("export-done", (e) => {
        setExporting(false);
        revealItemInDir(e.payload).catch(() => {});
      }),
    );
    unsubs.push(
      listen<string>("export-error", (e) => {
        setExporting(false);
        setExportErr(e.payload);
        if (lastFolderRef.current) revealItemInDir(`${lastFolderRef.current}\\video.mp4`).catch(() => {});
      }),
    );
    return () => {
      unsubs.forEach((u) => u.then((f) => f()));
    };
  }, [lastFolderRef]);

  return { exporting, pct, exportErr };
}
