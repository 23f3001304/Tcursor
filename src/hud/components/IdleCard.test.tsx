// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { IdleCard } from "./IdleCard";

vi.stubGlobal("matchMedia", () => ({
  matches: false,
  addEventListener() {},
  removeEventListener() {},
  addListener() {},
  removeListener() {},
}));

const targets = [
  { id: "display:0", label: "Display 1: \\\\.\\DISPLAY1 (2560x1440, Primary)", kind: "display" },
  { id: "display:1", label: "Display 2: \\\\.\\DISPLAY2 (1920x1080)", kind: "display" },
  { id: "window:0x1", label: "App: Notepad", kind: "window" },
];
const base: Parameters<typeof IdleCard>[0] = {
  banner: null,
  exporting: false,
  pct: 0,
  onOpenProject: vi.fn(),
  onPreferences: vi.fn(),
  onSettings: vi.fn(),
  onMinimize: vi.fn(),
  onClose: vi.fn(),
  camRef: () => {},
  camLive: false,
  cameras: [{ id: "c1", label: "Insta360" }],
  camId: "c1",
  onCam: vi.fn(),
  targets,
  displayId: "display:0",
  onTarget: vi.fn(),
  mics: [{ id: "m1", label: "Yeti X" }],
  micId: "m1",
  onMic: vi.fn(),
  menu: null,
  onMenu: vi.fn(),
  sheet: false,
  onSheet: vi.fn(),
  panel: null,
  panelBody: null,
  toggles: { camOn: true, micOn: true, sysOn: false, gameMode: false },
  onToggle: vi.fn(),
  onRecord: vi.fn(),
};

let host: HTMLDivElement;
let root: Root;
beforeEach(() => {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
});
afterEach(() => {
  act(() => root.unmount());
  host.remove();
  vi.clearAllMocks();
});
const render = (p: Partial<typeof base>) => act(() => root.render(<IdleCard {...base} {...p} />));
const byTitle = (t: string) => host.querySelector<HTMLButtonElement>(`button[title="${t}"]`);

describe("IdleCard", () => {
  it("stacks the preview, three source rows, the toggles and one Record", () => {
    render({});
    expect(host.querySelector(".camtoggle.wide")).not.toBeNull();
    expect(host.querySelectorAll(".dd-row")).toHaveLength(3);
    expect(host.querySelector(".dd-main .dd-label")?.textContent).toBe("Display 1: \\\\.\\DISPLAY1");
    expect(host.querySelector(".dd-sub")?.textContent).toBe("2560 by 1440 · Primary");
    expect(host.querySelectorAll(".tog")).toHaveLength(4);
    expect(host.querySelector(".btn.rec")?.textContent).toBe("Record");
  });

  it("the Screen row asks for the sheet", () => {
    render({});
    act(() => byTitle("Choose what to record")!.click());
    expect(base.onSheet).toHaveBeenCalledWith(true);
  });

  it("in the sheet, a pick selects and closes it; Back only closes", () => {
    render({ sheet: true });
    expect(host.querySelector(".sheet-title")?.textContent).toBe("What to record");
    expect(host.querySelectorAll(".sheet-list .tgt")).toHaveLength(3);
    expect(host.querySelector(".sheet-list .tgt .dd-sub")?.textContent).toBe("2560 by 1440 \u00b7 Primary");
    act(() => host.querySelectorAll<HTMLButtonElement>(".sheet-list .tgt")[1].click());
    expect(base.onTarget).toHaveBeenCalledWith("display:1");
    expect(base.onSheet).toHaveBeenLastCalledWith(false);
    act(() => byTitle("Back")!.click());
    expect(base.onSheet).toHaveBeenLastCalledWith(false);
  });

  it("a panel takes the card body, over the sheet, with the header still there", () => {
    render({ panel: "settings", panelBody: <div className="settings">panel</div>, sheet: true });
    expect(host.querySelector(".card-body .settings")?.textContent).toBe("panel");
    expect(host.querySelector(".sheet-title")).toBeNull();
    expect(host.querySelector(".camtoggle.wide")).toBeNull();
    expect(byTitle("Settings")).not.toBeNull();
    expect(byTitle("Close")).not.toBeNull();
  });

  it("toggles report their key; the system cluster hides its three app buttons while exporting", () => {
    render({});
    act(() => byTitle("System audio off")!.click());
    expect(base.onToggle).toHaveBeenCalledWith("sysOn");
    expect(byTitle("Settings")).not.toBeNull();
    render({ exporting: true, pct: 43 });
    expect(byTitle("Settings")).toBeNull();
    expect(host.querySelector<HTMLButtonElement>(".btn.rec")?.disabled).toBe(true);
    expect(host.querySelector(".btn.rec")?.textContent).toBe("Exporting… 43%");
  });
});
