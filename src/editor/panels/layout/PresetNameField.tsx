import { useState } from "react";
import { MAX_PRESET_NAME } from "./layoutPresets";

export function PresetNameField({
  initial,
  check,
  onCommit,
  onCancel,
}: {
  initial: string;
  check: (name: string) => string | null;
  onCommit: (name: string) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(initial);
  const [err, setErr] = useState<string | null>(null);
  const commit = () => {
    const problem = check(name);
    if (problem) {
      setErr(problem);
      return;
    }
    onCommit(name.trim());
  };
  return (
    <div className="e-lay-namewrap">
      <input
        className="e-lay-input"
        value={name}
        autoFocus
        maxLength={MAX_PRESET_NAME + 1}
        aria-label="Look name"
        aria-invalid={err !== null || undefined}
        onChange={(e) => {
          setName(e.target.value);
          setErr(null);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            commit();
          }
          if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
          }
        }}
      />
      <div className="e-lay-namebtns">
        <button type="button" className="e-lay-txt" onClick={commit}>
          Save
        </button>
        <button type="button" className="e-lay-txt" onClick={onCancel}>
          Cancel
        </button>
      </div>
      {err && (
        <p className="e-hintline e-lay-err" role="alert">
          {err}
        </p>
      )}
    </div>
  );
}
