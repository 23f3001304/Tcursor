import { describe, it, expect } from "vitest";
import { RIPPLE_CAP, dropRipple, pushRipple, rippleTone, suppressesRipple, type Ripple } from "./ripples";

const at = (id: number): Ripple => ({ id, x: id, y: id, tone: "rim" });

/** Build a detached tree from HTML and hand back the element matching `sel` - the pointerdown
 *  target the overlay would actually see (usually a glyph inside the button, not the button). */
function target(html: string, sel: string): Element {
  const host = document.createElement("div");
  host.innerHTML = html;
  const el = host.querySelector(sel);
  if (!el) throw new Error(`no ${sel} in fixture`);
  return el;
}

describe("ripple cap", () => {
  it("keeps every ripple until the cap and then evicts the oldest", () => {
    let list: Ripple[] = [];
    for (let i = 1; i <= RIPPLE_CAP; i++) list = pushRipple(list, at(i));
    expect(list.map((r) => r.id)).toEqual([1, 2, 3, 4, 5, 6]);
    list = pushRipple(list, at(7));
    expect(list).toHaveLength(RIPPLE_CAP);
    expect(list.map((r) => r.id)).toEqual([2, 3, 4, 5, 6, 7]); // oldest first, oldest out
  });

  it("never grows past the cap however fast the clicks come", () => {
    let list: Ripple[] = [];
    for (let i = 0; i < 200; i++) list = pushRipple(list, at(i));
    expect(list).toHaveLength(RIPPLE_CAP);
  });

  it("drops by id, and returns the same array when the id is already gone", () => {
    const list = [at(1), at(2), at(3)];
    expect(dropRipple(list, 2).map((r) => r.id)).toEqual([1, 3]);
    // Identity, not just equality: the overlay relies on React bailing out of this setState when
    // an already-evicted ripple's exit animation completes.
    expect(dropRipple(list, 99)).toBe(list);
  });
});

describe("opted-out surfaces", () => {
  it("suppresses a ripple anywhere inside a data-ui-fx=off wrapper", () => {
    const el = target(`<div data-ui-fx="off"><div class="e-stage"><canvas></canvas></div></div>`, "canvas");
    expect(suppressesRipple(el)).toBe(true);
  });

  it("suppresses on the marked element itself, not only on its children", () => {
    expect(suppressesRipple(target(`<div class="e-tracks" data-ui-fx="off"></div>`, ".e-tracks"))).toBe(true);
  });

  it("leaves ordinary chrome alone, and treats a null target as the editor's own ground", () => {
    expect(suppressesRipple(target(`<div class="e-panel"><button>Add</button></div>`, "button"))).toBe(false);
    expect(suppressesRipple(null)).toBe(false);
  });
});

describe("ripple tint", () => {
  it("tints the play button and Export with the accent", () => {
    expect(rippleTone(target(`<button class="e-play"><span class="e-play-glyph"></span></button>`, ".e-play-glyph"))).toBe("accent");
    expect(rippleTone(target(`<button class="e-export"><svg></svg></button>`, "svg"))).toBe("accent");
  });

  it("tints an engaged control with the accent, by `.on` or by aria-current", () => {
    expect(rippleTone(target(`<button class="e-tbtn on"><span>In</span></button>`, "span"))).toBe("accent");
    expect(rippleTone(target(`<button class="e-railbtn" aria-current="true"><svg></svg></button>`, "svg"))).toBe("accent");
  });

  it("leaves every quiet control on the neutral ring", () => {
    expect(rippleTone(target(`<button class="e-tbtn"><span>Out</span></button>`, "span"))).toBe("rim");
    expect(rippleTone(target(`<button class="e-tg"><svg></svg></button>`, "svg"))).toBe("rim");
    expect(rippleTone(null)).toBe("rim");
  });
});
