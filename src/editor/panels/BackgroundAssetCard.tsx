// The Background panel's "Your file" section: the import button that LEADS the row (it is what an
// empty section is for), and whatever is already imported sitting beside it.
//
// It is the LAST section of the Wallpapers tab, in the same stack as Ribbons, Folds and the rest -
// because importing IS one more way to choose a background, not a separate kind of object. The
// section's own header carries the name; this file renders only the row. Selecting the card
// re-asserts the asset's own kind, which is how switching to a wallpaper and back needs no
// re-import: the file never left.
import { useEffect, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { open } from "@tauri-apps/plugin-dialog";
import { IconPhotoPlus, IconTrash } from "@tabler/icons-react";
import type { BackgroundKind } from "../../hud/settings/settings";
import { backgroundAssetInfo, importBackgroundAsset, removeBackgroundAsset, fileSrc, type BackgroundAssetInfo } from "../../lib/ipc";
import { assetFileName, assetKindOf, assetSubtitle, thumbRel } from "./backgroundAsset";

const EXTENSIONS = ["png", "jpg", "jpeg", "webp", "gif", "mp4", "webm", "mov"];
// The panel's existing hint/error motion (CursorPanel's `HINT_MOTION`): one language, not a second.
const ENTER = { initial: { opacity: 0, y: -4 }, animate: { opacity: 1, y: 0 }, exit: { opacity: 0, y: -4 }, transition: { duration: 0.14 } };

export function BackgroundAssetCard({ folder, asset, kind, onPick, onImported, onRemoved }: {
  folder: string;
  /** `settings.background.asset`, kept even while a wallpaper shows. */
  asset: string | null | undefined;
  kind: BackgroundKind;
  onPick: (kind: "image" | "video") => void;
  onImported: (info: BackgroundAssetInfo) => void;
  onRemoved: () => void;
}) {
  // `undefined` = still asking; `null` = asked, and the file is gone.
  const [info, setInfo] = useState<BackgroundAssetInfo | null | undefined>(undefined);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState("");
  const still = useReducedMotion();

  useEffect(() => {
    if (!asset) { setInfo(undefined); return; }
    let live = true;
    setInfo(undefined);
    backgroundAssetInfo(folder, asset)
      .then((i) => { if (live) setInfo(i); })
      .catch(() => { if (live) setInfo(null); });
    return () => { live = false; };
  }, [folder, asset]);

  const handleImport = async () => {
    setErr("");
    const file = await open({ multiple: false, title: "Choose a background image or video",
      filters: [{ name: "Image or video", extensions: EXTENSIONS }] }).catch(() => null);
    if (!file || Array.isArray(file)) return;
    setBusy(true);
    try {
      const imported = await importBackgroundAsset(folder, file);
      setInfo(imported);
      onImported(imported);
    } catch (e) {
      setErr(typeof e === "string" ? e : "Import failed. Use a png, jpg, webp, gif, mp4, webm or mov file.");
    } finally { setBusy(false); }
  };

  const handleRemove = async () => {
    if (!asset) return;
    setErr("");
    try { await removeBackgroundAsset(folder, asset); } catch { /* already gone is still removed */ }
    setInfo(undefined);
    onRemoved();
  };

  const selected = kind === "image" || kind === "video";
  const assetKind = asset ? assetKindOf(asset) : null;
  const motionProps = still ? {} : ENTER;

  return (
    <div className="e-bgassetwrap">
      <div className="e-bgassetrow">
        {/* The import affordance leads the row at the imported card's own height, because
            importing IS one more way to choose a background rather than a separate mode. */}
        <button type="button" className="e-bgadd" onClick={handleImport} disabled={busy}
          aria-label="Import image or video"
          title={busy ? "Importing..." : "Choose an image or video from this computer"}>
          <IconPhotoPlus size={16} />
        </button>
        <AnimatePresence initial={false}>
          {asset && (
            <motion.div key={asset} className={`e-bgasset ${selected ? "on" : ""}`} {...motionProps}>
              <button type="button" className="e-bgasset-pick" aria-pressed={selected}
                title={assetFileName(asset)} disabled={!assetKind}
                onClick={() => assetKind && onPick(assetKind)}>
                <span className="e-bgasset-thumb"
                  style={{ backgroundImage: `url(${fileSrc(`${folder}\\${thumbRel(asset).split("/").join("\\")}`)})` }} />
                <span className="e-bgasset-text">
                  <span className="e-bgasset-name">{assetFileName(asset)}</span>
                  <span className="e-bgasset-sub">{assetSubtitle(info)}</span>
                </span>
              </button>
              <button type="button" className="e-bgasset-rm" onClick={handleRemove}
                aria-label={`Remove ${assetFileName(asset)}`} title="Remove">
                <IconTrash size={14} />
              </button>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
      <AnimatePresence>
        {err && <motion.p className="e-errline" {...motionProps}>{err}</motion.p>}
      </AnimatePresence>
    </div>
  );
}
