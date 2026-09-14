import { useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { IconDots, IconPlus } from "@tabler/icons-react";
import type { AppearanceSettings, LayoutPreset } from "../../hud/settings/settings";
import { BUILTIN_PRESET, MAX_PRESET_NAME, presetNameError } from "./layoutPresets";

// The 0.16s content-swap tween the panels collapse and swap on - one motion language.
const TWEEN = { type: "tween" as const, duration: 0.16, ease: [0.4, 0, 0.2, 1] as const };

/** The one text field this section has, used for both "save the current look" and "rename this
 *  look". Enter commits, Esc cancels, and a refused name says why underneath instead of silently
 *  doing nothing - the caller supplies the reason through `check`, so the same field enforces the
 *  same rule in both places. Autofocuses: it only ever exists because the user just asked for it. */
function NameField({ initial, check, onCommit, onCancel }: {
  initial: string;
  check: (name: string) => string | null;
  onCommit: (name: string) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(initial);
  const [err, setErr] = useState<string | null>(null);
  const commit = () => {
    const problem = check(name);
    if (problem) { setErr(problem); return; }
    onCommit(name.trim());
  };
  return (
    <div className="e-lay-namewrap">
      <input className="e-lay-input" value={name} autoFocus maxLength={MAX_PRESET_NAME + 1}
        aria-label="Look name" aria-invalid={err !== null || undefined}
        onChange={(e) => { setName(e.target.value); setErr(null); }}
        onKeyDown={(e) => {
          if (e.key === "Enter") { e.preventDefault(); commit(); }
          if (e.key === "Escape") { e.preventDefault(); onCancel(); }
        }} />
      <div className="e-lay-namebtns">
        <button type="button" className="e-lay-txt" onClick={commit}>Save</button>
        <button type="button" className="e-lay-txt" onClick={onCancel}>Cancel</button>
      </div>
      {err && <p className="e-hintline e-lay-err" role="alert">{err}</p>}
    </div>
  );
}

/** The saved-looks list: the built-in row, then one row per saved look, then the two write actions
 *  (save the current look, make it the default for new recordings). A look is a snapshot of ALL
 *  FIVE layouts, so applying one is a single write of `settings.appearance` - which is why Apply is
 *  a plain button per row rather than a selected state: nothing here is "current", because the
 *  moment a knob moves the project has drifted from whatever was applied. */
export function LayoutPresetList({ presets, appearance, onApply, onSave, onRename, onDelete, onMakeDefault }: {
  presets: LayoutPreset[];
  /** The project's current look - what "Save current look" and "Make default" both write. */
  appearance: AppearanceSettings;
  onApply: (p: LayoutPreset) => void;
  onSave: (name: string) => void;
  onRename: (id: string, name: string) => void;
  onDelete: (id: string) => void;
  onMakeDefault: (a: AppearanceSettings) => void;
}) {
  const still = useReducedMotion();
  const timing = still ? { duration: 0 } : TWEEN;
  // Exactly one of these is open at a time: the new-look field, or one row's rename field, or one
  // row's actions. A panel this narrow cannot afford two open things fighting for the same 320px.
  const [adding, setAdding] = useState(false);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [menu, setMenu] = useState<string | null>(null);
  const [madeDefault, setMadeDefault] = useState(false);
  const close = () => { setAdding(false); setRenaming(null); setMenu(null); };

  const rows = [BUILTIN_PRESET, ...presets];

  return (
    <div className="e-grp">
      <span className="e-sechead">Saved looks</span>
      <div className="e-lay-list">
        <AnimatePresence initial={false}>
          {rows.map((p) => {
            const builtin = p.id === BUILTIN_PRESET.id;
            return (
              <motion.div key={p.id} className="e-lay-row" layout={!still}
                initial={still ? false : { opacity: 0, height: 0 }} animate={{ opacity: 1, height: "auto" }}
                exit={{ opacity: 0, height: 0, transition: timing }} transition={timing}>
                {renaming === p.id ? (
                  <NameField initial={p.name} check={(n) => presetNameError(n, presets, p.id)}
                    onCommit={(n) => { onRename(p.id, n); close(); }} onCancel={close} />
                ) : (
                  <>
                    <div className="e-lay-rowtop">
                      <span className="e-lay-name" title={p.name}>{p.name}</span>
                      <button type="button" className="e-lay-txt" onClick={() => { close(); onApply(p); }}>Apply</button>
                      {!builtin && (
                        <button type="button" className="e-lay-kebab" aria-label={`More for ${p.name}`}
                          aria-expanded={menu === p.id} onClick={() => setMenu(menu === p.id ? null : p.id)}>
                          <IconDots size={14} />
                        </button>
                      )}
                    </div>
                    <AnimatePresence initial={false}>
                      {menu === p.id && (
                        <motion.div className="e-lay-acts" key="acts"
                          initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: "auto" }}
                          exit={{ opacity: 0, height: 0 }} transition={timing}>
                          <button type="button" className="e-lay-txt"
                            onClick={() => { setMenu(null); setAdding(false); setRenaming(p.id); }}>Rename</button>
                          <button type="button" className="e-lay-txt"
                            onClick={() => { close(); onDelete(p.id); }}>Delete</button>
                        </motion.div>
                      )}
                    </AnimatePresence>
                  </>
                )}
              </motion.div>
            );
          })}
        </AnimatePresence>
      </div>

      {/* Deliberately NOT `mode="wait"`: the field must exist the instant the button is pressed
          (the button is what the user is looking at, and it autofocuses the field), so the two
          cross-fade over each other rather than the field waiting for the button to finish. */}
      <AnimatePresence initial={false}>
        {adding ? (
          <motion.div key="add" initial={still ? false : { opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }} exit={{ opacity: 0, height: 0 }} transition={timing}>
            <NameField initial="" check={(n) => presetNameError(n, presets)}
              onCommit={(n) => { onSave(n); close(); }} onCancel={close} />
          </motion.div>
        ) : (
          <motion.button key="addbtn" type="button" className="e-upload"
            initial={still ? false : { opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
            transition={timing} onClick={() => { close(); setAdding(true); }}>
            <IconPlus size={14} /> Save current look
          </motion.button>
        )}
      </AnimatePresence>

      <button type="button" className="e-lay-txt e-lay-default"
        onClick={() => { onMakeDefault(appearance); setMadeDefault(true); }}>
        Make this the default for new recordings
      </button>
      <AnimatePresence initial={false}>
        {madeDefault && (
          <motion.p className="e-hintline" key="dflt" initial={{ opacity: 0 }} animate={{ opacity: 1 }}
            exit={{ opacity: 0 }} transition={timing}>
            New recordings start with this look. Apply Default and press it again to undo.
          </motion.p>
        )}
      </AnimatePresence>
    </div>
  );
}
