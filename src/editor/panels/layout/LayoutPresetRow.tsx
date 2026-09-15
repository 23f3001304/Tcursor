import type { Transition } from "motion/react";
import { AnimatePresence, motion } from "motion/react";
import { IconDots } from "@tabler/icons-react";
import type { LayoutPreset } from "../../../hud/settings/settings";
import { BUILTIN_PRESET } from "./layoutPresets";
import { PresetNameField } from "./PresetNameField";

export function LayoutPresetRow({
  preset,
  still,
  timing,
  renaming,
  menuOpen,
  checkName,
  onApply,
  onRename,
  onDelete,
  onOpenMenu,
  onStartRename,
  onClose,
}: {
  preset: LayoutPreset;
  still: boolean;
  timing: Transition;
  renaming: boolean;
  menuOpen: boolean;
  checkName: (name: string) => string | null;
  onApply: () => void;
  onRename: (name: string) => void;
  onDelete: () => void;
  onOpenMenu: () => void;
  onStartRename: () => void;
  onClose: () => void;
}) {
  const builtin = preset.id === BUILTIN_PRESET.id;
  return (
    <motion.div
      className="e-lay-row"
      layout={!still}
      initial={still ? false : { opacity: 0, height: 0 }}
      animate={{ opacity: 1, height: "auto" }}
      exit={{ opacity: 0, height: 0, transition: timing }}
      transition={timing}
    >
      {renaming ? (
        <PresetNameField initial={preset.name} check={checkName} onCommit={onRename} onCancel={onClose} />
      ) : (
        <>
          <div className="e-lay-rowtop">
            <span className="e-lay-name" title={preset.name}>
              {preset.name}
            </span>
            <button type="button" className="e-lay-txt" onClick={onApply}>
              Apply
            </button>
            {!builtin && (
              <button
                type="button"
                className="e-lay-kebab"
                aria-label={`More for ${preset.name}`}
                aria-expanded={menuOpen}
                onClick={onOpenMenu}
              >
                <IconDots size={14} />
              </button>
            )}
          </div>
          <AnimatePresence initial={false}>
            {menuOpen && (
              <motion.div
                className="e-lay-acts"
                key="acts"
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: "auto" }}
                exit={{ opacity: 0, height: 0 }}
                transition={timing}
              >
                <button type="button" className="e-lay-txt" onClick={onStartRename}>
                  Rename
                </button>
                <button type="button" className="e-lay-txt" onClick={onDelete}>
                  Delete
                </button>
              </motion.div>
            )}
          </AnimatePresence>
        </>
      )}
    </motion.div>
  );
}
