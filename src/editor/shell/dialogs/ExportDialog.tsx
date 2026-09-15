import { useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { IconDownload, IconX } from "@tabler/icons-react";
import { Picker } from "../../controls/fields/Picker";
import { Slider } from "../../controls/fields/Slider";
import type { ExportSettings, ExportResolution, ExportFps, ExportFormat } from "../../../shared/ipc";
import { DEFAULT_EXPORT_SETTINGS } from "../../../shared/ipc";
import { ExportProgress } from "./ExportProgress";

const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

const RESOLUTIONS: { value: ExportResolution; label: string }[] = [
  { value: "source", label: "Source (recording size)" },
  { value: "p720", label: "720p" },
  { value: "p1080", label: "1080p" },
  { value: "p1440", label: "1440p" },
  { value: "p2160", label: "4K (2160p)" },
];
const FPS_OPTIONS: { value: ExportFps; label: string }[] = [
  { value: "source", label: "Source (capture rate)" },
  { value: "f30", label: "30 fps" },
  { value: "f60", label: "60 fps" },
];
const FORMATS: { value: ExportFormat; label: string }[] = [
  { value: "mp4", label: "MP4 (H.264)" },
  { value: "webm", label: "WebM (VP9)" },
  { value: "gif", label: "GIF" },
];

export function ExportDialog({
  open,
  exporting,
  pct,
  done,
  error,
  exportPath,
  startedAt,
  onClose,
  onExport,
  onReset,
}: {
  open: boolean;
  exporting: boolean;
  pct: number;
  done: boolean;
  error: string | null;
  exportPath: string;
  startedAt: number | null;
  onClose: () => void;
  onExport: (settings: ExportSettings) => void;
  onReset: () => void;
}) {
  const [settings, setSettings] = useState<ExportSettings>(DEFAULT_EXPORT_SETTINGS);
  const handleClose = () => {
    if (done || error) onReset();
    onClose();
  };
  const showProgress = exporting || done || !!error;
  const isGif = settings.format === "gif";

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="e-modal-scrim"
          onPointerDown={handleClose}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.14 }}
        >
          <motion.div
            className="e-modal e-export-modal"
            onPointerDown={(e) => e.stopPropagation()}
            initial={{ opacity: 0, scale: 0.96, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: 8 }}
            transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
          >
            <div className="e-export-head">
              <h3 className="e-modal-title">Export video</h3>
              <button type="button" className="e-gst" title="Close" onClick={handleClose}>
                <IconX size={16} />
              </button>
            </div>

            {showProgress ? (
              <ExportProgress
                exporting={exporting}
                pct={pct}
                done={done}
                error={error}
                exportPath={exportPath}
                startedAt={startedAt}
                onReset={onReset}
                onClose={handleClose}
              />
            ) : (
              <>
                <div className="e-export-row">
                  <label>Resolution</label>
                  <Picker
                    value={settings.resolution}
                    options={RESOLUTIONS}
                    onChange={(resolution) => setSettings((s) => ({ ...s, resolution }))}
                  />
                </div>
                <div className="e-export-row">
                  <label>Frame rate</label>
                  <Picker
                    value={settings.fps}
                    options={FPS_OPTIONS}
                    onChange={(fps) => setSettings((s) => ({ ...s, fps }))}
                  />
                </div>
                <div className="e-export-row">
                  <label>Format</label>
                  <Picker
                    value={settings.format}
                    options={FORMATS}
                    onChange={(format) => setSettings((s) => ({ ...s, format }))}
                  />
                </div>
                <div className="e-export-row">
                  <label>Quality{isGif ? "" : ` (CRF ${settings.quality_crf})`}</label>
                  <Slider
                    value={settings.quality_crf}
                    min={18}
                    max={28}
                    step={1}
                    disabled={isGif}
                    onChange={(quality_crf) => setSettings((s) => ({ ...s, quality_crf }))}
                    ariaLabel="Quality"
                  />
                  <p className="e-export-hint">
                    {isGif
                      ? "Not used for GIF - quality comes from the color palette."
                      : "Lower = higher quality, larger file."}
                  </p>
                </div>
                <div className="e-modal-actions">
                  <button type="button" className="e-modal-btn" onClick={handleClose}>
                    Cancel
                  </button>
                  <motion.button
                    type="button"
                    className="e-modal-btn primary"
                    onClick={() => onExport(settings)}
                    whileTap={PRESS_TAP}
                    transition={PRESS_SPRING}
                  >
                    <IconDownload size={14} />
                    Export
                  </motion.button>
                </div>
              </>
            )}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
