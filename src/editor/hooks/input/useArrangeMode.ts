import { useCallback, useEffect, useRef, useState } from "react";
import type { EditDoc, LayoutSeg } from "../../../shared/edit";
import { arrangeSeekMs, nextArrangeMode, NO_ARRANGE, type ArrangeMode } from "../../model/arrangeMode";

export function useArrangeMode(
  doc: EditDoc | null,
  sel: string | null,
  io: {
    setSel: (id: string | null) => void;
    timeMsRef: { current: number };
    onSeek: (ms: number) => void;
  },
): {
  arrangeSeg: LayoutSeg | null;
  arrangeOn: boolean;
  onArrange: () => void;
  onSel: (id: string | null) => void;
} {
  const ioRef = useRef(io);
  ioRef.current = io;
  const [mode, setMode] = useState<ArrangeMode>(NO_ARRANGE);
  const docRef = useRef(doc);
  docRef.current = doc;
  const isLayout = (id: string | null) => (id && docRef.current?.layout.some((l) => l.id === id) ? id : null);
  const selSegId = isLayout(sel);
  useEffect(() => {
    setMode((m) => nextArrangeMode(m, { kind: "select", segId: selSegId }));
  }, [selSegId]);

  const seg = mode.segId ? (doc?.layout.find((l) => l.id === mode.segId) ?? null) : null;
  const gone = mode.segId !== null && seg === null;
  useEffect(() => {
    if (gone) setMode((m) => nextArrangeMode(m, { kind: "gone" }));
  }, [gone]);

  useEffect(() => {
    if (!mode.on) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMode((m) => nextArrangeMode(m, { kind: "escape" }));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [mode.on]);

  const segRef = useRef(seg);
  segRef.current = seg;
  useEffect(() => {
    const s = segRef.current;
    if (!mode.on || !s) return;
    const ms = arrangeSeekMs(s, ioRef.current.timeMsRef.current);
    if (ms !== null) ioRef.current.onSeek(ms);
  }, [mode.on, mode.segId]); // eslint-disable-line react-hooks/exhaustive-deps

  const onArrange = useCallback(() => setMode((m) => nextArrangeMode(m, { kind: "arrange" })), []);
  const onSel = useCallback((id: string | null) => {
    ioRef.current.setSel(id);
    setMode((m) => nextArrangeMode(m, { kind: "select", segId: isLayout(id) }));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps
  return { arrangeSeg: mode.on ? seg : null, arrangeOn: mode.on, onArrange, onSel };
}
