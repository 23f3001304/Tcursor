// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { StageMedia } from "./StageMedia";

let root: Root;
let container: HTMLDivElement;
let seeked = 0;

const noop = () => {};

const mount = (wantsScreenB: boolean) =>
  act(() => {
    root.render(
      <StageMedia
        screenRef={{ current: null }}
        screenBRef={{ current: null }}
        wantsScreenB={wantsScreenB}
        webcamRef={{ current: null }}
        audioRef={{ current: null }}
        src="screen.mp4"
        webcamSrc=""
        audioSrc=""
        err={null}
        onRetry={noop}
        onScreenLoadedData={noop}
        onScreenSeeked={() => {
          seeked += 1;
        }}
        onScreenEnded={noop}
        onScreenLoadedMetadata={noop}
        onScreenError={noop}
        onWebcamLoadedData={noop}
        onWebcamSeeked={noop}
      />,
    );
  });

const videos = () => [...container.querySelectorAll("video")];

const fireSeeked = (el: Element) =>
  act(() => {
    el.dispatchEvent(new Event("seeked"));
  });

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  seeked = 0;
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

describe("StageMedia", () => {
  it("mounts the dissolve's second screen video only when the document can dissolve", () => {
    mount(false);
    expect(videos().length).toBe(1);
    mount(true);
    expect(videos().length).toBe(2);
  });

  it("repaints when the second video's pre-seek lands, exactly as the first one does", () => {
    mount(true);
    fireSeeked(videos()[0]);
    expect(seeked).toBe(1);
    fireSeeked(videos()[1]);
    expect(seeked).toBe(2);
  });
});
