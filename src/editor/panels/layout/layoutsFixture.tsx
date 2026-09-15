import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { EditDoc, LayoutSeg } from "../../../shared/edit";
import type { Settings } from "../../../hud/settings/settings";
import { DEFAULT_APPEARANCE } from "../../../hud/preferences/appearanceFields";

export const BOLD = { ...DEFAULT_APPEARANCE, presenter: { ...DEFAULT_APPEARANCE.presenter, pad: 0.07 } };

export const h = {
  container: null as unknown as HTMLDivElement,
  app: null as unknown as Settings,
  written: [] as Settings[],
  saved: [] as EditDoc["settings"][],
};

export const ipcMock = () => ({
  getSettings: () => Promise.resolve(h.app),
  setSettings: (s: Settings) => {
    h.written.push(s);
    return Promise.resolve();
  },
});

const seg = (layout: string): LayoutSeg => ({
  id: "l1",
  start_ms: 0,
  end_ms: 2000,
  layout,
  transition_ms: 0,
  easing: "smooth",
  transition_out_ms: 0,
  easing_out: "smooth",
});

export const docWith = (layout: string): EditDoc =>
  ({
    layout: [seg(layout)],
    settings: { ai_model: "", appearance: DEFAULT_APPEARANCE, layout_presets: [] },
  }) as unknown as EditDoc;

export const byText = <T extends HTMLElement>(sel: string, text: string) =>
  Array.from(h.container.querySelectorAll<T>(sel)).find((e) => e.textContent?.trim() === text)!;

export const rowNames = () =>
  Array.from(h.container.querySelectorAll(".e-lay-name")).map((e) => e.textContent);

let root: Root;

export function mountPanel() {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  h.saved = [];
  h.written = [];
  h.app = { ai_model: "", appearance: DEFAULT_APPEARANCE, layout_presets: [] } as unknown as Settings;
  localStorage.clear();
  h.container = document.createElement("div");
  document.body.appendChild(h.container);
  root = createRoot(h.container);
}

export function unmountPanel() {
  act(() => {
    root.unmount();
  });
  h.container.remove();
  localStorage.clear();
}

// The panel is imported lazily: the ipc mock's factory pulls this module in, so a static import of
// the panel (which imports ipc) would close the cycle and deadlock the runner.
export const show = async (doc: EditDoc, timeMs = 500) => {
  const { LayoutsPanel } = await import("./LayoutsPanel");
  await act(async () => {
    root.render(
      <LayoutsPanel
        doc={doc}
        timeMsRef={{ current: timeMs }}
        onSaveSettings={(s) => h.saved.push(s)}
        onClose={() => {}}
      />,
    );
  });
};
