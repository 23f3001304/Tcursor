// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { act } from "react";
import type { Clip } from "../../shared/edit";
import { clipsFixtureMap } from "../../shared/math/remap.fixture";
import { q, qa, show, useInspectorDom } from "./inspectorFixture";
import { ClipInspector } from "./ClipInspector";

useInspectorDom();

const clips: Clip[] = [
  { id: "cl1", src_in_ms: 6000, src_out_ms: 9000, transition_in_ms: 0 },
  { id: "cl0", src_in_ms: 500, src_out_ms: 4000, transition_in_ms: 500 },
];

function insp(id: string, patch: Partial<Clip> = {}) {
  const onApply = vi.fn().mockResolvedValue(null);
  const onClose = vi.fn();
  const list = clips.map((c) => (c.id === id ? { ...c, ...patch } : c));
  show(
    <ClipInspector
      clip={list.find((c) => c.id === id)!}
      clips={list}
      map={clipsFixtureMap()}
      dur={10_000}
      motionEasing="smooth"
      onApply={onApply}
      onClose={onClose}
    />,
  );
  return { onApply, onClose };
}

const startMore = () => q<HTMLButtonElement>(".e-field2 .e-field .e-numstep[title^='More']");
const endLess = () => qa<HTMLButtonElement>(".e-field2 .e-field .e-numstep[title^='Less']")[1];
const hints = () => qa(".e-ihint").map((e) => e.textContent ?? "");

describe("ClipInspector", () => {
  it("names the clip by its place in the export and shows both ranges", () => {
    insp("cl0");
    expect(q(".e-ihead h2")?.textContent).toBe("Clip 2");
    expect(q(".e-ihead-range")?.textContent).toBe("0.50s to 4.00s");
    expect(hints().some((t) => /5\.00s to 7\.00s/.test(t))).toBe(true);
  });

  it("commits a source retime as update_clip", () => {
    const { onApply } = insp("cl0");
    act(() => {
      startMore()?.click();
    });
    expect(onApply).toHaveBeenCalledWith({ op: "update_clip", id: "cl0", src_in_ms: 600 });
  });

  it("stops the two steppers a clip floor apart, the same 100ms the Clips lane keeps", () => {
    insp("cl0", { src_in_ms: 3900 });
    expect(startMore()?.disabled).toBe(true);
    insp("cl0", { src_out_ms: 600 });
    expect(endLess()?.disabled).toBe(true);
  });

  it("never sends an update that would collapse the clip to nothing", () => {
    const { onApply } = insp("cl0", { src_in_ms: 3850 });
    act(() => {
      startMore()?.click();
    });
    expect(onApply).not.toHaveBeenCalled();
  });

  it("caps the transition slider at what the op will allow", () => {
    insp("cl0");
    expect(q('[aria-label="Dissolve in"]')?.getAttribute("aria-valuemax")).toBe("1000");
  });

  it("says nothing dissolves into the first clip", () => {
    insp("cl1");
    expect(q('[aria-label="Dissolve in"]')).toBeNull();
    expect(hints().some((t) => /first clip/i.test(t))).toBe(true);
  });

  it("removes through remove_clip and closes", () => {
    const { onApply, onClose } = insp("cl0");
    act(() => {
      q<HTMLButtonElement>('[title="Remove clip 2"]')?.click();
    });
    expect(onApply).toHaveBeenCalledWith({ op: "remove_clip", id: "cl0" });
    expect(onClose).toHaveBeenCalled();
  });
});
