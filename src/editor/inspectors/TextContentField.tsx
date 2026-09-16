import { IconX } from "@tabler/icons-react";
import type { TextItem, TextKind } from "../../shared/edit";
import { Picker } from "../controls/Controls";
import { TEXT_KIND_OPTIONS } from "../panels/textStyles";

export function TextContentField({
  item,
  onText,
  onSub,
  onKind,
}: {
  item: TextItem;
  onText: (text: string) => void;
  onSub: (sub: string | null) => void;
  onKind: (kind: TextKind) => void;
}) {
  return (
    <>
      <textarea
        className="e-itextarea"
        aria-label="Text"
        rows={2}
        spellCheck
        value={item.text}
        onChange={(e) => onText(e.target.value)}
        onKeyDown={(e) => e.stopPropagation()}
      />
      <div className="e-isubrow">
        <input
          className="e-isubinput"
          aria-label="Second line"
          placeholder="Second line"
          value={item.sub ?? ""}
          onChange={(e) => onSub(e.target.value)}
          onKeyDown={(e) => e.stopPropagation()}
        />
        <button
          type="button"
          className="e-isubclear"
          title="Clear the second line"
          aria-label="Clear the second line"
          disabled={!item.sub}
          onClick={() => onSub(null)}
        >
          <IconX size={13} />
        </button>
      </div>
      <div className="e-field">
        <span className="e-fl">Kind</span>
        <Picker value={item.kind} options={TEXT_KIND_OPTIONS} onChange={onKind} ariaLabel="Kind" />
      </div>
    </>
  );
}
