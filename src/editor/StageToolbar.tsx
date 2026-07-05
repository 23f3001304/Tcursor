import { IconAspectRatio, IconClick, IconTypography, IconVideo } from "@tabler/icons-react";

export function StageToolbar() {
  return (
    <div className="e-ftool">
      <button title="Aspect ratio"><IconAspectRatio size={18} /></button>
      <button title="Cursor"><IconClick size={18} /></button>
      <button title="Captions"><IconTypography size={18} /></button>
      <button title="3D camera"><IconVideo size={18} /></button>
    </div>
  );
}
