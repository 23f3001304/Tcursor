import { motion } from "motion/react";
import { IconDownload, IconTextCaption } from "@tabler/icons-react";
import { Picker, Segmented } from "../../controls/Controls";
import { Spin } from "../../controls/surfaces/Spin";
import type { CaptionStyle } from "../../../hud/settings/settings";
import type { WhisperModelDto } from "../../../shared/ipc";
import { actionFor, languageOptions, modelOptionLabel, type Phase } from "./modelCopy";

const SWAP_TWEEN = { type: "tween" as const, duration: 0.16, ease: [0.4, 0, 0.2, 1] as const };

export function TranscribeCard({
  models,
  style,
  phase,
  pct,
  error,
  onStyle,
  onDownload,
  onTranscribe,
}: {
  models: WhisperModelDto[];
  style: CaptionStyle;
  phase: Phase;
  pct: number;
  error: string | null;
  onStyle: (s: CaptionStyle) => void;
  onDownload: (id: string) => void;
  onTranscribe: () => void;
}) {
  const picked = models.find((m) => m.id === style.model) ?? null;
  const action = actionFor(picked, phase);
  const langs = languageOptions(picked);
  const blocked = langs.find((o) => o.disabled);
  const press = () => {
    if (action.kind === "download" && picked) onDownload(picked.id);
    else if (action.kind === "transcribe") onTranscribe();
  };
  const pickModel = (id: string) => {
    const next = models.find((m) => m.id === id);
    const language = next?.multilingual ? style.language : "en";
    onStyle({ ...style, model: id, language });
  };

  return (
    <div className="e-grp">
      <span className="e-sechead">Transcribe</span>
      <div className="e-field">
        <span className="e-fl">Model</span>
        <Picker
          value={style.model}
          ariaLabel="Model"
          onChange={pickModel}
          options={models.map((m) => ({ value: m.id, label: modelOptionLabel(m), title: m.id }))}
        />
      </div>
      {langs.length - (blocked ? 1 : 0) > 1 && (
        <div className="e-field">
          <span className="e-fl">Language</span>
          <Segmented
            value={style.language}
            ariaLabel="Language"
            onChange={(v) => onStyle({ ...style, language: v })}
            options={langs.filter((o) => !o.disabled).map((o) => ({ value: o.value, label: o.label }))}
          />
        </div>
      )}
      {blocked?.title && <p className="e-hintline">{blocked.title}</p>}

      <button type="button" className="e-run e-caprun" onClick={press} disabled={action.disabled}>
        {action.disabled && action.kind === "busy" ? (
          <>
            <Spin size={16} />
            {action.label}
          </>
        ) : (
          <>
            {action.kind === "download" ? <IconDownload size={16} /> : <IconTextCaption size={16} />}
            {action.label}
          </>
        )}
      </button>

      {phase !== "idle" && (
        <>
          <div
            className="e-capbar"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={pct}
            aria-label={action.label}
          >
            <motion.div
              className="e-capbar-fill"
              initial={false}
              animate={{ width: `${Math.max(0, Math.min(100, pct))}%` }}
              transition={SWAP_TWEEN}
            />
          </div>
          <p className="e-capstate">
            {action.label}, {pct}%
          </p>
        </>
      )}
      {error && (
        <p className="e-errline" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
