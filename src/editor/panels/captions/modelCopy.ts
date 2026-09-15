import type { WhisperModelDto } from "../../../shared/ipc";

export type Phase = "idle" | "downloading" | "decoding" | "transcribing";

export interface Action {
  kind: "download" | "transcribe" | "busy" | "pick";
  label: string;
  disabled: boolean;
}

export const sizeWarning = (bytes: number) => `${Math.round(bytes / 1_048_576)} MB download, once`;

export function actionFor(model: WhisperModelDto | null, phase: Phase): Action {
  if (phase === "downloading") return { kind: "busy", label: "Downloading", disabled: true };
  if (phase === "decoding") return { kind: "busy", label: "Reading the audio", disabled: true };
  if (phase === "transcribing") return { kind: "busy", label: "Transcribing", disabled: true };
  if (!model) return { kind: "pick", label: "Pick a model", disabled: true };
  return model.installed
    ? { kind: "transcribe", label: "Transcribe audio", disabled: false }
    : { kind: "download", label: "Download model", disabled: false };
}

export function languageOptions(
  model: WhisperModelDto | null,
): { value: string; label: string; disabled: boolean; title?: string }[] {
  const multi = model?.multilingual ?? false;
  return [
    { value: "en", label: "English", disabled: false },
    {
      value: "auto",
      label: "Auto detect",
      disabled: !multi,
      title: multi
        ? undefined
        : "Auto detect needs a multilingual model. Switch the model to Base (multilingual).",
    },
  ];
}

export const modelOptionLabel = (m: WhisperModelDto) =>
  m.installed ? m.label : `${m.label} (${sizeWarning(m.bytes)})`;
