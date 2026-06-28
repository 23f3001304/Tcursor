import { invoke } from "@tauri-apps/api/core";
import type { DisplayInfo, AudioInfo } from "../hud/selectDevices";
import type { Settings } from "../hud/settings";

export const listDisplays = () => invoke<DisplayInfo[]>("list_displays");
export const listAudioInputs = () => invoke<AudioInfo[]>("list_audio_inputs");
export const startRecording = (projectName: string, micId: string | null, systemAudio: boolean, gameMode: boolean) =>
  invoke<void>("start_recording", { projectName, micId, systemAudio, gameMode });
export const pauseRecording = () => invoke<void>("pause_recording");
export const resumeRecording = () => invoke<void>("resume_recording");
export const stopRecording = () =>
  invoke<{ folder: string; frames: number }>("stop_recording");
export const saveWebcam = (folder: string, bytes: number[]) =>
  invoke<void>("save_webcam", { folder, bytes });
export const exportProject = (folder: string) =>
  invoke<void>("export_project", { folder });
export const getSettings = () => invoke<Settings>("get_settings");
export const setSettings = (settings: Settings) => invoke<void>("set_settings", { settings });
