import { useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { IconPlus } from "@tabler/icons-react";
import type { AppearanceSettings, LayoutPreset } from "../../../hud/settings/settings";
import { BUILTIN_PRESET, presetNameError } from "./layoutPresets";
import { LayoutPresetRow } from "./LayoutPresetRow";
import { PresetNameField } from "./PresetNameField";

const TWEEN = { type: "tween" as const, duration: 0.16, ease: [0.4, 0, 0.2, 1] as const };

export function LayoutPresetList({
  presets,
  appearance,
  onApply,
  onSave,
  onRename,
  onDelete,
  onMakeDefault,
}: {
  presets: LayoutPreset[];
  appearance: AppearanceSettings;
  onApply: (p: LayoutPreset) => void;
  onSave: (name: string) => void;
  onRename: (id: string, name: string) => void;
  onDelete: (id: string) => void;
  onMakeDefault: (a: AppearanceSettings) => void;
}) {
  const still = useReducedMotion();
  const timing = still ? { duration: 0 } : TWEEN;
  const [adding, setAdding] = useState(false);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [menu, setMenu] = useState<string | null>(null);
  const [madeDefault, setMadeDefault] = useState(false);
  const close = () => {
    setAdding(false);
    setRenaming(null);
    setMenu(null);
  };

  const rows = [BUILTIN_PRESET, ...presets];

  return (
    <div className="e-grp">
      <span className="e-sechead">Saved looks</span>
      <div className="e-lay-list">
        <AnimatePresence initial={false}>
          {rows.map((p) => (
            <LayoutPresetRow
              key={p.id}
              preset={p}
              still={still === true}
              timing={timing}
              renaming={renaming === p.id}
              menuOpen={menu === p.id}
              checkName={(n) => presetNameError(n, presets, p.id)}
              onApply={() => {
                close();
                onApply(p);
              }}
              onRename={(n) => {
                onRename(p.id, n);
                close();
              }}
              onDelete={() => {
                close();
                onDelete(p.id);
              }}
              onOpenMenu={() => setMenu(menu === p.id ? null : p.id)}
              onStartRename={() => {
                setMenu(null);
                setAdding(false);
                setRenaming(p.id);
              }}
              onClose={close}
            />
          ))}
        </AnimatePresence>
      </div>

      <AnimatePresence initial={false}>
        {adding ? (
          <motion.div
            key="add"
            initial={still ? false : { opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={timing}
          >
            <PresetNameField
              initial=""
              check={(n) => presetNameError(n, presets)}
              onCommit={(n) => {
                onSave(n);
                close();
              }}
              onCancel={close}
            />
          </motion.div>
        ) : (
          <motion.button
            key="addbtn"
            type="button"
            className="e-upload"
            initial={still ? false : { opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={timing}
            onClick={() => {
              close();
              setAdding(true);
            }}
          >
            <IconPlus size={14} /> Save current look
          </motion.button>
        )}
      </AnimatePresence>

      <button
        type="button"
        className="e-lay-txt e-lay-default"
        onClick={() => {
          onMakeDefault(appearance);
          setMadeDefault(true);
        }}
      >
        Make this the default for new recordings
      </button>
      <AnimatePresence initial={false}>
        {madeDefault && (
          <motion.p
            className="e-hintline"
            key="dflt"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={timing}
          >
            New recordings start with this look. Apply Default and press it again to undo.
          </motion.p>
        )}
      </AnimatePresence>
    </div>
  );
}
