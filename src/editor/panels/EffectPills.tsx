import { IconZoomIn, IconBulb, IconAspectRatio, IconVideo } from "@tabler/icons-react";

// Native HTML5 drag: plain draggable divs, NOT motion.div - Motion's gesture layer (pointer
// capture) intercepts the drag and stops dragstart from firing (commit b23b73d - `whileHover`
// alone was already enough to break it). The hover lift on `.e-libpill` below is therefore a
// plain CSS transition, not the Motion spring the rest of this pass reaches for - see that
// class's own comment in editor.css.
const handleDragStart = (e: React.DragEvent, type: string) => {
  e.dataTransfer.setData("text/plain", type);
  e.dataTransfer.effectAllowed = "copy";
};

export function EffectPills({
  onAddZoom,
  onAddSpotlight,
  onAddLayout,
  onAddCameraMove,
}: {
  onAddZoom: () => void;
  onAddSpotlight: () => void;
  onAddLayout: () => void;
  onAddCameraMove: () => void;
}) {
  return (
    <div className="e-grp">
      <span className="e-sechead">Insert timeline elements</span>
      <div className="e-libgrid">

        <div draggable onDragStart={(e) => handleDragStart(e, "layout")} onClick={onAddLayout}
          className="e-layblk e-libpill">
          <IconAspectRatio size={16} className="e-libicon" />
          <span className="e-libtext">
            <span className="e-libname">Layout Segment</span>
            <span className="e-libhint">Switch the frame layout</span>
          </span>
        </div>

        <div draggable onDragStart={(e) => handleDragStart(e, "zoom")} onClick={onAddZoom}
          className="e-zblk e-libpill">
          <IconZoomIn size={16} className="e-libicon" />
          <span className="e-libtext">
            <span className="e-libname">Zoom Region</span>
            <span className="e-libhint">Push in on a click or region</span>
          </span>
        </div>

        <div draggable onDragStart={(e) => handleDragStart(e, "spotlight")} onClick={onAddSpotlight}
          className="e-fxblk e-libpill">
          <IconBulb size={16} className="e-libicon" />
          <span className="e-libtext">
            <span className="e-libname">Spotlight Highlight</span>
            <span className="e-libhint">Dim everything but the focus</span>
          </span>
        </div>

        <div draggable onDragStart={(e) => handleDragStart(e, "cammove")} onClick={onAddCameraMove}
          className="e-camkfpill e-libpill">
          <IconVideo size={16} className="e-libicon" />
          <span className="e-libtext">
            <span className="e-libname">Camera Move</span>
            <span className="e-libhint">Keyframe the webcam</span>
          </span>
        </div>

      </div>
    </div>
  );
}
