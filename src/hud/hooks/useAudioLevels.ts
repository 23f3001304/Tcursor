import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

/** Payload of the Rust `audio-level` event (`src-tauri/src/audio/level.rs`). */
interface AudioLevel { source: "mic" | "system"; rms: number }

/** A source is treated as live only while its reports keep arriving. The Rust threads report every
 *  ~50ms, so three missed windows plus slack is decisive without being twitchy. */
const STALE_MS = 400;

/** Live 0..1 RMS for the recorder's two audio sources, straight from the capture that is writing
 *  the WAV - no second `getUserMedia` stream, so the meter cannot show a level for audio the take
 *  is not actually recording, and the app never opens the microphone twice.
 *
 *  Returns a `read()` getter rather than state: at 20 reports a second per source, storing these
 *  in React state would re-render the whole recording bar forty times a second to move a wave the
 *  meter's own rAF loop is already redrawing. `read` is referentially stable, so a memoised meter
 *  never re-subscribes. `live` IS state - it changes a handful of times per take (a device
 *  opening, a driver resetting, a mic toggled off) and gates the meter's honest greyed-out look. */
export function useAudioLevels(on: boolean): { read: () => { mic: number; sys: number }; live: boolean } {
  const levels = useRef({ mic: 0, sys: 0, micAt: 0, sysAt: 0 });
  const [live, setLive] = useState(false);

  const read = useCallback(() => {
    const now = performance.now();
    const l = levels.current;
    return {
      mic: now - l.micAt < STALE_MS ? l.mic : 0,
      sys: now - l.sysAt < STALE_MS ? l.sys : 0,
    };
  }, []);

  useEffect(() => {
    levels.current = { mic: 0, sys: 0, micAt: 0, sysAt: 0 };
    if (!on) { setLive(false); return; }
    const un = listen<AudioLevel>("audio-level", (e) => {
      const now = performance.now();
      const l = levels.current;
      if (e.payload.source === "mic") { l.mic = e.payload.rms; l.micAt = now; }
      else { l.sys = e.payload.rms; l.sysAt = now; }
      setLive(true);
    });
    // A take can lose its input mid-recording (device unplugged, driver reset, another app taking
    // exclusive access). The reports simply stop, so nothing else would notice: poll for that
    // silence and drop `live`, rather than leaving a frozen wave that still implies capture.
    const stale = setInterval(() => {
      const now = performance.now();
      const l = levels.current;
      setLive(now - l.micAt < STALE_MS || now - l.sysAt < STALE_MS);
    }, 250);
    return () => { clearInterval(stale); void un.then((f) => f()); };
  }, [on]);

  return { read, live };
}
