import type { ComponentType } from "react";
import { IconSparkles, IconPhoto, IconPointer, IconCamera, IconLetterCase, IconVolume, IconWand } from "@tabler/icons-react";

export type Tab = "ai" | "background" | "cursor" | "camera" | "captions" | "audio" | "effects";

export const PANEL_TABS: { id: Tab; icon: ComponentType<{ size?: number }>; label: string }[] = [
  { id: "ai", icon: IconSparkles, label: "AI Director" },
  { id: "background", icon: IconPhoto, label: "Background" },
  { id: "cursor", icon: IconPointer, label: "Cursor" },
  { id: "camera", icon: IconCamera, label: "Camera" },
  { id: "captions", icon: IconLetterCase, label: "Captions" },
  { id: "audio", icon: IconVolume, label: "Audio" },
  { id: "effects", icon: IconWand, label: "Effects" },
];
