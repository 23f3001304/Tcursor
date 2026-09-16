// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { GradeSettings } from "../../../hud/settings/settings";
import { GradeSection, NO_GRADE } from "./GradeSection";

let root: Root, container: HTMLDivElement;
let grade: GradeSettings;

const show = () => {
  act(() => {
    root.render(
      <GradeSection
        grade={grade}
        onChange={(next) => {
          grade = next;
          show();
        }}
      />,
    );
  });
};

const lookLabel = () =>
  container.querySelector<HTMLButtonElement>('[aria-label="Look"]')!.querySelector("span")!.textContent;
const pickLook = (name: string) => {
  act(() => {
    container.querySelector<HTMLButtonElement>('[aria-label="Look"]')!.click();
  });
  const opt = Array.from(document.body.querySelectorAll<HTMLButtonElement>('[role="option"]')).find(
    (o) => o.textContent === name,
  )!;
  act(() => {
    opt.click();
  });
};
const nudgeExposure = () => {
  const slider = container.querySelector<HTMLElement>('[role="slider"][aria-label="Exposure"]')!;
  act(() => {
    slider.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
  });
};

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  grade = NO_GRADE;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => {
  act(() => {
    root.unmount();
  });
  container.remove();
});

describe("GradeSection's Custom state", () => {
  it("reads Custom once a knob drifts off the picked look, and the look again when re-picked", () => {
    show();
    pickLook("Noir");
    expect(grade).toEqual({ preset: "noir", exposure: 0.05, contrast: 1.3, vignette: 0.4 });
    expect(lookLabel()).toBe("Noir");

    nudgeExposure();
    expect(grade.preset).toBe("noir");
    expect(grade.exposure).toBeCloseTo(0.1, 6);
    expect(lookLabel()).toBe("Custom");

    pickLook("Noir");
    expect(grade).toEqual({ preset: "noir", exposure: 0.05, contrast: 1.3, vignette: 0.4 });
    expect(lookLabel()).toBe("Noir");
  });

  it("never reads Custom on the None look, whose knobs are the user's own", () => {
    show();
    expect(lookLabel()).toBe("None");
    nudgeExposure();
    expect(grade.exposure).toBeCloseTo(0.05, 6);
    expect(lookLabel()).toBe("None");
  });
});
