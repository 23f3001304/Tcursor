import { useEffect, useRef, useState } from "react";

export function CaptionTextField({
  value,
  onCommit,
  ariaLabel = "Caption text",
}: {
  value: string;
  onCommit: (text: string) => void;
  ariaLabel?: string;
}) {
  const [draft, setDraft] = useState(value);
  const editing = useRef(false);
  useEffect(() => {
    if (!editing.current) setDraft(value);
  }, [value]);

  const commit = () => {
    editing.current = false;
    const next = draft.trim();
    if (next === "" || next === value.trim()) {
      setDraft(value);
      return;
    }
    onCommit(next);
  };

  return (
    <textarea
      className="e-captext"
      aria-label={ariaLabel}
      rows={2}
      value={draft}
      spellCheck
      onFocus={() => {
        editing.current = true;
      }}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          e.currentTarget.blur();
        } else if (e.key === "Escape") {
          e.preventDefault();
          editing.current = false;
          setDraft(value);
          e.currentTarget.blur();
        }
        e.stopPropagation();
      }}
    />
  );
}
