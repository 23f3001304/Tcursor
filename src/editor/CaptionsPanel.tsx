import { useState, useEffect } from "react";
import { IconX, IconSparkles, IconTrash } from "@tabler/icons-react";
import type { ClickFxSettings } from "../hud/settings";
import { Switch } from "./Controls";

interface Subtitle {
  id: string;
  start: string;
  end: string;
  text: string;
}

const INITIAL_SUBTITLES: Subtitle[] = [
  { id: "1", start: "0:01", end: "0:04", text: "Welcome to this quick walkthrough." },
  { id: "2", start: "0:04", end: "0:08", text: "Today we are looking at layout designs." },
  { id: "3", start: "0:08", end: "0:12", text: "First, let's select a custom cursor." },
  { id: "4", start: "0:12", end: "0:16", text: "See how the motion trail adapts." },
  { id: "5", start: "0:16", end: "0:20", text: "Next, we will add click effects." },
];

export function CaptionsPanel({
  settings,
  onChange,
  onClose,
}: {
  settings: ClickFxSettings;
  onChange: (v: ClickFxSettings) => void;
  onClose: () => void;
}) {
  const [generating, setGenerating] = useState(false);
  const [progress, setProgress] = useState(0);
  const [subtitles, setSubtitles] = useState<Subtitle[]>([]);

  const set = <K extends keyof ClickFxSettings>(k: K, v: ClickFxSettings[K]) => {
    onChange({ ...settings, [k]: v });
  };

  useEffect(() => {
    if (!generating) return;
    const interval = setInterval(() => {
      setProgress((p) => {
        if (p >= 100) {
          clearInterval(interval);
          setGenerating(false);
          setSubtitles(INITIAL_SUBTITLES);
          return 100;
        }
        return p + 20;
      });
    }, 200);
    return () => clearInterval(interval);
  }, [generating]);

  const handleGenerate = () => {
    setGenerating(true);
    setProgress(0);
    setSubtitles([]);
  };

  return (
    <div className="e-panel e-insp">
      <div className="e-insphdr">
        <h2>Subtitles & Captions</h2>
        <button className="e-gst" title="Close" onClick={onClose}><IconX size={16} /></button>
      </div>
      <p className="e-lede">Generate real-time subtitles and configure caption overlay rules.</p>

      {/* Auto-generate block */}
      {!generating && subtitles.length === 0 && (
        <div style={{ padding: "10px 0" }}>
          <button className="e-run" onClick={handleGenerate}>
            <IconSparkles size={16} /> Auto-Generate Subtitles
          </button>
        </div>
      )}

      {/* Loading state */}
      {generating && (
        <div style={{ padding: "16px 0", textAlign: "center" }}>
          <span style={{ fontSize: 13, color: "var(--e-mut)" }}>Transcribing audio stream...</span>
          <div style={{ height: 4, background: "var(--e-soft)", borderRadius: 2, overflow: "hidden", margin: "12px 0 6px" }}>
            <div style={{ width: `${progress}%`, height: "100%", background: "var(--e-primary)", transition: "width 0.15s ease" }} />
          </div>
          <span style={{ fontSize: 11, color: "var(--e-dim)" }}>{progress}% complete</span>
        </div>
      )}

      {/* Timed subtitle blocks - using native .e-sum list style */}
      {!generating && subtitles.length > 0 && (
        <div className="e-field">
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
            <span className="e-fl" style={{ margin: 0 }}>Editable Captions</span>
            <button className="e-gst" style={{ color: "var(--accent, #ef4444)", fontSize: 12 }} onClick={() => setSubtitles([])}>Clear</button>
          </div>
          
          <div className="e-sublist">
            {subtitles.map((sub, idx) => (
              <div key={sub.id} className="e-subrow">
                <span className="e-subtime">
                  {sub.start}
                </span>
                <input
                  type="text"
                  className="e-subinput"
                  value={sub.text}
                  onChange={(e) => {
                    const updated = [...subtitles];
                    updated[idx].text = e.target.value;
                    setSubtitles(updated);
                  }}
                />
                <button
                  type="button"
                  onClick={() => setSubtitles(subtitles.filter(s => s.id !== sub.id))}
                  className="e-subdel"
                >
                  <IconTrash size={14} />
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Keystrokes Overlay switch */}
      <div className="e-field" style={{ borderTop: "1px solid var(--e-border2)", paddingTop: 16, marginTop: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={{ fontSize: 13, color: "var(--e-fg)" }}>Show keystrokes on screen</span>
          <Switch on={settings.captions} onChange={(v) => set("captions", v)} />
        </div>
      </div>
    </div>
  );
}
