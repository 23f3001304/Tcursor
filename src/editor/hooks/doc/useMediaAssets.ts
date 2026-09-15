import { useCallback, useEffect, useRef, useState } from "react";
import {
  DEFAULT_PROXY_HEIGHT,
  ensurePreviewAudio,
  ensureProxy,
  ensureThumbs,
  ensureWaveform,
  fileSrc,
  getProjectManifest,
} from "../../../shared/ipc";
import { planProxySrc } from "../../model/editorData";
import { FILMSTRIP_COUNT, FILMSTRIP_HEIGHT } from "../../timeline/model/filmstripPlan";

export function useMediaAssets(folder: string, quality: number, setPlaying: (p: boolean) => void) {
  const [thumbs, setThumbs] = useState<string[]>([]);
  const [waves, setWaves] = useState<{ system: string; mic: string }>({ system: "", mic: "" });
  const [wavesReady, setWavesReady] = useState(false);
  const [audioUrl, setAudioUrl] = useState("");
  const [srcUrl, setSrcUrl] = useState("");
  const [manifest, setManifest] = useState({ ready: false, preprocessed: false });

  useEffect(() => {
    let live = true;
    setManifest({ ready: false, preprocessed: false });
    getProjectManifest(folder)
      .then((m) => {
        if (live) setManifest({ ready: true, preprocessed: m.preprocessed });
      })
      .catch(() => {
        if (live) setManifest({ ready: true, preprocessed: false });
      });
    return () => {
      live = false;
    };
  }, [folder]);

  useEffect(() => {
    let live = true;
    setWavesReady(false);
    ensureThumbs(folder, FILMSTRIP_COUNT, FILMSTRIP_HEIGHT)
      .then((p) => {
        if (live) setThumbs(p.map(fileSrc));
      })
      .catch(() => {});
    Promise.all([
      ensureWaveform(folder, "system").catch(() => ""),
      ensureWaveform(folder, "mic").catch(() => ""),
    ])
      .then(([s, m]) => {
        if (live) setWaves({ system: s ? fileSrc(s) : "", mic: m ? fileSrc(m) : "" });
      })
      .catch(() => {})
      .finally(() => {
        if (live) setWavesReady(true);
      });
    ensurePreviewAudio(folder)
      .then((p) => {
        if (live) setAudioUrl(p ? fileSrc(p) : "");
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [folder]);

  const proxyReadyRef = useRef(false);
  const lastFolderRef = useRef(folder);
  const [reloadTick, setReloadTick] = useState(0);
  const retryMedia = useCallback(() => setReloadTick((t) => t + 1), []);
  useEffect(() => {
    setPlaying(false);
    if (lastFolderRef.current !== folder) {
      lastFolderRef.current = folder;
      proxyReadyRef.current = false;
    }
    if (!manifest.ready) return;
    const plan = planProxySrc(quality, DEFAULT_PROXY_HEIGHT, manifest.preprocessed, proxyReadyRef.current);
    if (plan.immediate) setSrcUrl(fileSrc(`${folder}\\${plan.immediate}`));
    if (plan.known) {
      setSrcUrl(fileSrc(`${folder}\\${plan.known}`));
      proxyReadyRef.current = true;
      return;
    }
    if (!plan.fetch) return;
    let live = true;
    ensureProxy(folder, quality)
      .then((p) => {
        if (live) {
          setSrcUrl(fileSrc(p));
          proxyReadyRef.current = true;
        }
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [folder, quality, manifest, reloadTick]);

  return { thumbs, waves, wavesReady, audioUrl, srcUrl, retryMedia };
}
