import type { CSSProperties, RefObject, SyntheticEvent } from "react";

const HIDDEN: CSSProperties = { position: "absolute", width: 1, height: 1, opacity: 0, pointerEvents: "none" };

/** The Stage's hidden native media: the screen/webcam `<video>`s `useCompositeLoop` reads every
 *  frame via canvas.drawImage (never actually shown - HIDDEN keeps them decoding off-canvas),
 *  the mixed preview `<audio>`, and the "preview unavailable" error overlay. Extracted from
 *  Stage.tsx to keep that file under the line limit; all state (the refs themselves, `err`,
 *  dirty-tracking) still lives in Stage - this component only renders what it's handed. */
export function StageMedia({
  screenRef, webcamRef, audioRef, src, webcamSrc, audioSrc, err,
  onScreenLoadedData, onScreenSeeked, onScreenEnded, onScreenLoadedMetadata, onScreenError,
  onWebcamLoadedData, onWebcamSeeked,
}: {
  screenRef: RefObject<HTMLVideoElement | null>;
  webcamRef: RefObject<HTMLVideoElement | null>;
  audioRef: RefObject<HTMLAudioElement | null>;
  src: string; webcamSrc: string; audioSrc: string; err: string | null;
  onScreenLoadedData: () => void;
  onScreenSeeked: () => void;
  onScreenEnded: () => void;
  onScreenLoadedMetadata: (e: SyntheticEvent<HTMLVideoElement>) => void;
  onScreenError: (e: SyntheticEvent<HTMLVideoElement>) => void;
  onWebcamLoadedData: () => void;
  onWebcamSeeked: () => void;
}) {
  return (
    <>
      {src && (
        <video ref={screenRef} src={src} muted playsInline preload="auto" style={HIDDEN}
          onLoadedData={onScreenLoadedData} onSeeked={onScreenSeeked} onEnded={onScreenEnded}
          onLoadedMetadata={onScreenLoadedMetadata} onError={onScreenError} />
      )}
      {webcamSrc && <video ref={webcamRef} src={webcamSrc} muted playsInline preload="auto" style={HIDDEN}
        onLoadedData={onWebcamLoadedData} onSeeked={onWebcamSeeked} />}
      {audioSrc && <audio ref={audioRef} src={audioSrc} preload="auto" />}
      {err && <div className="e-stage-empty" style={{ position: "absolute", inset: 0 }}>Preview unavailable - {err}</div>}
    </>
  );
}
