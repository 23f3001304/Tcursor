import { invoke } from "@tauri-apps/api/core";
import type { DisplayInfo, AudioInfo } from "../hud/selectDevices";

export const listDisplays = () => invoke<DisplayInfo[]>("list_displays");
export const listAudioInputs = () => invoke<AudioInfo[]>("list_audio_inputs");
export const startRecording = (projectName: string, micId: string | null, systemAudio: boolean) =>
  invoke<void>("start_recording", { projectName, micId, systemAudio });
export const pauseRecording = () => invoke<void>("pause_recording");
export const resumeRecording = () => invoke<void>("resume_recording");
export const stopRecording = () =>
  invoke<{ folder: string; frames: number }>("stop_recording");
export const saveWebcam = (folder: string, bytes: number[]) =>
  invoke<void>("save_webcam", { folder, bytes });
export const exportProject = (folder: string) =>
  invoke<void>("export_project", { folder });
