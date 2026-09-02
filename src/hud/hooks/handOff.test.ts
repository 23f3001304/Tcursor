import { describe, it, expect } from "vitest";
import { handOff } from "./handOff";

// Fix round 1, item 2 (controller ruling 2026-09-02): `finish` used to call `onEdit` without
// awaiting it, then reset `saving` on the very next tick - racing `App.openEditor`'s own async
// window resize on the SAME OS window. `handOff` is the extracted piece that makes `finish` await
// the handoff instead; these tests pin the invariant directly with a fake `onEdit`, without
// rendering the hook or mocking Tauri/IPC.
describe("handOff", () => {
  it("resolves true (finish should reset saving) when no onEdit was provided - standalone mode", async () => {
    await expect(handOff(undefined, "C:/rec-1")).resolves.toBe(true);
  });

  it("awaits onEdit and resolves false (finish should LEAVE saving true) once it succeeds", async () => {
    let called: string | null = null;
    const onEdit = async (folder: string) => { called = folder; };
    await expect(handOff(onEdit, "C:/rec-1")).resolves.toBe(false);
    expect(called).toBe("C:/rec-1");
  });

  it("does not settle until onEdit's OWN promise settles - the whole point of the fix, so nothing " +
    "in `finish` can reset `saving` (and re-arm the camera gate) while onEdit's window resize is " +
    "still in flight", async () => {
    let releaseOnEdit: () => void = () => {};
    const onEdit = () => new Promise<void>((resolve) => { releaseOnEdit = resolve; });
    let settled = false;
    const pending = handOff(onEdit, "C:/rec-1").then((v) => { settled = true; return v; });
    // Drain a few microtask turns without releasing onEdit - handOff must still be pending.
    await Promise.resolve(); await Promise.resolve(); await Promise.resolve();
    expect(settled).toBe(false);
    releaseOnEdit();
    await expect(pending).resolves.toBe(false);
    expect(settled).toBe(true);
  });

  it("propagates a rejected onEdit instead of swallowing it - finish's own resetSaving default (true) is what restores idle on this path", async () => {
    const onEdit = async () => { throw new Error("resize failed"); };
    await expect(handOff(onEdit, "C:/rec-1")).rejects.toThrow("resize failed");
  });
});
