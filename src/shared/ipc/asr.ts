import { invoke } from "@tauri-apps/api/core";

export interface WhisperModelDto {
  id: string;
  label: string;
  bytes: number;
  installed: boolean;
  multilingual: boolean;
}

export const whisperModels = () => invoke<WhisperModelDto[]>("whisper_models");

export const downloadWhisperModel = (id: string) => invoke<void>("download_whisper_model", { id });

export interface AsrDownloadProgress {
  id: string;
  done: number;
  total: number;
}

export const transcribeProject = (folder: string) => invoke<void>("transcribe_project", { folder });

export interface AsrProgress {
  phase: "decode" | "transcribe";
  pct: number;
}
