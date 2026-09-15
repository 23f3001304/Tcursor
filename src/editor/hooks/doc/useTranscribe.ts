import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { downloadWhisperModel, transcribeProject, whisperModels } from "../../../shared/ipc";
import type { AsrDownloadProgress, AsrProgress, WhisperModelDto } from "../../../shared/ipc";
import type { Phase } from "../../panels/captions/modelCopy";

export interface Transcribe {
  models: WhisperModelDto[];
  refreshModels: () => void;
  phase: Phase;
  pct: number;
  error: string | null;
  download: (id: string) => void;
  transcribe: () => void;
}

export function useTranscribe(folder: string, onDone?: () => void): Transcribe {
  const [models, setModels] = useState<WhisperModelDto[]>([]);
  const [phase, setPhase] = useState<Phase>("idle");
  const [pct, setPct] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const doneRef = useRef(onDone);
  doneRef.current = onDone;

  const refreshModels = useCallback(() => {
    whisperModels()
      .then(setModels)
      .catch(() => {});
  }, []);
  useEffect(() => {
    refreshModels();
  }, [refreshModels]);

  useEffect(() => {
    let live = true;
    const offs: (() => void)[] = [];
    const on = <T>(name: string, cb: (p: T) => void) => {
      void listen<T>(name, (e) => {
        if (live) cb(e.payload);
      }).then((off) => {
        if (live) offs.push(off);
        else off();
      });
    };
    on<AsrDownloadProgress>("asr-download-progress", (p) => {
      setPhase("downloading");
      setPct(p.total > 0 ? Math.round((p.done / p.total) * 100) : 0);
    });
    on<string>("asr-download-done", () => {
      setPhase("idle");
      setPct(0);
      refreshModels();
    });
    on<string>("asr-download-error", (m) => {
      setPhase("idle");
      setError(m);
    });
    on<AsrProgress>("asr-progress", (p) => {
      setPhase(p.phase === "decode" ? "decoding" : "transcribing");
      setPct(Math.max(0, Math.min(100, p.pct)));
    });
    on<number>("asr-done", () => {
      setPhase("idle");
      setPct(0);
      doneRef.current?.();
    });
    on<string>("asr-error", (m) => {
      setPhase("idle");
      setError(m);
    });
    return () => {
      live = false;
      for (const off of offs) off();
    };
  }, [refreshModels]);

  const download = useCallback((id: string) => {
    setError(null);
    setPhase("downloading");
    setPct(0);
    downloadWhisperModel(id).catch((e: unknown) => {
      setPhase("idle");
      setError(String(e));
    });
  }, []);

  const transcribe = useCallback(() => {
    setError(null);
    setPhase("decoding");
    setPct(0);
    transcribeProject(folder).catch((e: unknown) => {
      setPhase("idle");
      setError(String(e));
    });
  }, [folder]);

  return { models, refreshModels, phase, pct, error, download, transcribe };
}
