import { useState } from "react";
import { PanelHeader } from "../PanelHeader";
import { ConfirmDialog } from "../../controls/surfaces/ConfirmDialog";
import { CaptionList } from "./CaptionList";
import { CaptionStyleControls, DEFAULT_CAPTION_STYLE } from "./CaptionStyleControls";
import { TranscribeCard } from "./TranscribeCard";
import { useTranscribe } from "../../hooks/doc/useTranscribe";
import type { EditDoc, EditOp } from "../../../shared/edit";

export function CaptionsPanel({
  folder,
  doc,
  applyOp,
  saveDocSettings,
  reloadDoc,
  timeMs,
  sel,
  onSeek,
  onSel,
  onClose,
}: {
  folder: string;
  doc: EditDoc;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  saveDocSettings: (s: EditDoc["settings"]) => void;
  reloadDoc: () => void;
  timeMs: number;
  sel: string | null;
  onSeek: (ms: number) => void;
  onSel: (id: string | null) => void;
  onClose: () => void;
}) {
  const asr = useTranscribe(folder, reloadDoc);
  const [clearing, setClearing] = useState(false);
  const style = doc.settings.captions;
  const caps = doc.captions;
  const setStyle = (captions: typeof style) => saveDocSettings({ ...doc.settings, captions });

  return (
    <div className="e-panel e-insp e-cappanel">
      <PanelHeader
        title="Captions"
        lede="Transcribe what you say, then edit it on the timeline."
        onReset={() => setStyle({ ...DEFAULT_CAPTION_STYLE, model: style.model, language: style.language })}
        onClose={onClose}
      />

      <TranscribeCard
        models={asr.models}
        style={style}
        phase={asr.phase}
        pct={asr.pct}
        error={asr.error}
        onStyle={setStyle}
        onDownload={asr.download}
        onTranscribe={asr.transcribe}
      />

      <CaptionStyleControls style={style} accent={doc.settings.ui.accent} onChange={setStyle} />

      <div className="e-grp e-capgrp">
        <span className="e-sechead">
          {caps.length === 0 ? "Transcript" : `${caps.length} caption${caps.length === 1 ? "" : "s"}`}
        </span>
        {caps.length === 0 ? (
          <p className="e-hintline">
            Nothing transcribed yet. Run the model above and the lines land here and on the timeline.
          </p>
        ) : (
          <>
            <CaptionList
              captions={caps}
              timeMs={timeMs}
              sel={sel}
              onPick={(c) => {
                onSeek(c.start_ms);
                onSel(c.id);
              }}
            />
            <button type="button" className="e-lay-txt" onClick={() => setClearing(true)}>
              Clear all captions
            </button>
          </>
        )}
      </div>

      <ConfirmDialog
        open={clearing}
        title="Clear every caption?"
        body="The whole transcript goes. Transcribing again re-creates it from the recording's own audio, but any text you edited by hand is gone."
        confirmLabel="Clear captions"
        danger
        onCancel={() => setClearing(false)}
        onConfirm={() => {
          setClearing(false);
          onSel(null);
          void applyOp({ op: "clear_captions" });
        }}
      />
    </div>
  );
}
