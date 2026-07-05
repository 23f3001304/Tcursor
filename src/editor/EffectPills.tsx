import { IconZoomIn, IconBulb, IconAspectRatio } from "@tabler/icons-react";

const handleDragStart = (e: React.DragEvent, type: string) => {
  e.dataTransfer.setData("text/plain", type);
  e.dataTransfer.effectAllowed = "copy";
};

// Drag & Drop Elements Grid: the layout/zoom/spotlight pills that add a timeline element
// on click or can be dragged onto the timeline.
export function EffectPills({
  onAddZoom,
  onAddSpotlight,
  onAddLayout,
}: {
  onAddZoom: () => void;
  onAddSpotlight: () => void;
  onAddLayout: () => void;
}) {
  return (
    <div className="e-field">
      <span className="e-fl">Insert timeline elements</span>
      <div style={{ display: "flex", flexDirection: "column", gap: 10, marginTop: 4 }}>

        {/* Layout Pill */}
        <div
          draggable
          onDragStart={(e) => handleDragStart(e, "layout")}
          onClick={onAddLayout}
          className="e-fxblk"
          style={{
            position: "relative",
            top: 0,
            left: 0,
            width: "100%",
            height: 34,
            cursor: "grab",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            background: "linear-gradient(180deg, #7c6cf0, #5b5bd6)"
          }}
        >
          <span className="e-zlabel" style={{ color: "#fff", display: "flex", alignItems: "center", gap: 6, pointerEvents: "none" }}>
            <IconAspectRatio size={14} /> Layout Segment
          </span>
        </div>

        {/* Zoom Pill */}
        <div
          draggable
          onDragStart={(e) => handleDragStart(e, "zoom")}
          onClick={onAddZoom}
          className="e-zblk"
          style={{
            position: "relative",
            top: 0,
            left: 0,
            width: "100%",
            height: 34,
            cursor: "grab",
            display: "flex",
            alignItems: "center",
            justifyContent: "center"
          }}
        >
          <span className="e-zlabel" style={{ color: "#fff", display: "flex", alignItems: "center", gap: 6, pointerEvents: "none" }}>
            <IconZoomIn size={14} /> Zoom Region
          </span>
        </div>

        {/* Spotlight Pill */}
        <div
          draggable
          onDragStart={(e) => handleDragStart(e, "spotlight")}
          onClick={onAddSpotlight}
          className="e-fxblk"
          style={{
            position: "relative",
            top: 0,
            left: 0,
            width: "100%",
            height: 34,
            cursor: "grab",
            display: "flex",
            alignItems: "center",
            justifyContent: "center"
          }}
        >
          <span className="e-zlabel" style={{ display: "flex", alignItems: "center", gap: 6, pointerEvents: "none" }}>
            <IconBulb size={14} /> Spotlight Highlight
          </span>
        </div>

      </div>
    </div>
  );
}
