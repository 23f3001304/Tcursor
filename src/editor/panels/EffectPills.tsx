import { IconZoomIn, IconBulb, IconAspectRatio, IconVideo } from "@tabler/icons-react";

// Native HTML5 drag: plain draggable divs, NOT motion.div - Motion's gesture layer
// (pointer capture) intercepts the drag and stops dragstart from firing. Hover-scale is CSS.
const handleDragStart = (e: React.DragEvent, type: string) => {
  e.dataTransfer.setData("text/plain", type);
  e.dataTransfer.effectAllowed = "copy";
};

// Full-width variants of the flat timeline pills (zoom=violet, spotlight=amber, layout=teal):
// the base lane class supplies the flat fill + border; drag onto the timeline, or click to add.
const pillStyle: React.CSSProperties = {
  position: "relative", top: 0, left: 0, width: "100%", height: 32, cursor: "grab",
  display: "flex", alignItems: "center", justifyContent: "center", transition: "transform .12s ease",
};
const labelStyle: React.CSSProperties = {
  color: "#fff", display: "flex", alignItems: "center", gap: 6, pointerEvents: "none",
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
    <div className="e-field">
      <span className="e-fl">Insert timeline elements</span>
      <div style={{ display: "flex", flexDirection: "column", gap: 10, marginTop: 4 }}>

        <div draggable onDragStart={(e) => handleDragStart(e, "layout")} onClick={onAddLayout}
          className="e-layblk e-libpill" style={pillStyle}>
          <span className="e-zlabel" style={labelStyle}><IconAspectRatio size={14} /> Layout Segment</span>
        </div>

        <div draggable onDragStart={(e) => handleDragStart(e, "zoom")} onClick={onAddZoom}
          className="e-zblk e-libpill" style={pillStyle}>
          <span className="e-zlabel" style={labelStyle}><IconZoomIn size={14} /> Zoom Region</span>
        </div>

        <div draggable onDragStart={(e) => handleDragStart(e, "spotlight")} onClick={onAddSpotlight}
          className="e-fxblk e-libpill" style={pillStyle}>
          <span className="e-zlabel" style={labelStyle}><IconBulb size={14} /> Spotlight Highlight</span>
        </div>

        <div draggable onDragStart={(e) => handleDragStart(e, "cammove")} onClick={onAddCameraMove}
          className="e-camkfpill e-libpill" style={pillStyle}>
          <span className="e-zlabel" style={labelStyle}><IconVideo size={14} /> Camera Move</span>
        </div>

      </div>
    </div>
  );
}
