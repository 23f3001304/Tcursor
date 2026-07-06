import { IconX, IconRotate2 } from "@tabler/icons-react";

// Shared panel/inspector header: title + a hairline, with matched ghost icon buttons
// (reset + close) so every panel reads the same. Reset is optional.
export function PanelHeader({ title, lede, onReset, onClose, closeTitle = "Close" }: {
  title: string;
  lede?: string;
  onReset?: () => void;
  onClose: () => void;
  closeTitle?: string;
}) {
  return (
    <div className="e-phead">
      <div className="e-phead-top">
        <h2>{title}</h2>
        <div className="e-hicons">
          {onReset && (
            <button type="button" className="e-hicon" title="Reset to defaults" aria-label="Reset to defaults" onClick={onReset}>
              <IconRotate2 size={15} />
            </button>
          )}
          <button type="button" className="e-hicon" title={closeTitle} aria-label={closeTitle} onClick={onClose}>
            <IconX size={15} />
          </button>
        </div>
      </div>
      {lede && <p className="e-lede">{lede}</p>}
    </div>
  );
}
