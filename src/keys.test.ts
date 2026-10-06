import { describe, expect, it } from "vitest";
import { keyAction } from "./keys";

const mac = (key: string, mods: { ctrl?: boolean; meta?: boolean } = {}) =>
  keyAction({ key, ctrlKey: !!mods.ctrl, metaKey: !!mods.meta, isMac: true });
const win = (key: string, mods: { ctrl?: boolean; meta?: boolean } = {}) =>
  keyAction({ key, ctrlKey: !!mods.ctrl, metaKey: !!mods.meta, isMac: false });

describe("keyAction", () => {
  it("maps zoom shortcuts to Cmd on macOS and Ctrl on Windows", () => {
    expect(mac("=", { meta: true })).toBe("zoom-in");
    expect(mac("+", { meta: true })).toBe("zoom-in");
    expect(mac("-", { meta: true })).toBe("zoom-out");
    expect(mac("0", { meta: true })).toBe("zoom-reset");
    expect(mac("=", { ctrl: true })).toBeNull();
    expect(win("=", { ctrl: true })).toBe("zoom-in");
    expect(win("-", { ctrl: true })).toBe("zoom-out");
    expect(win("0", { ctrl: true })).toBe("zoom-reset");
    expect(win("=", { meta: true })).toBeNull();
  });

  it("maps full screen keys per platform", () => {
    expect(mac("f", { ctrl: true, meta: true })).toBe("fullscreen");
    expect(mac("F", { ctrl: true, meta: true })).toBe("fullscreen");
    expect(mac("F11")).toBeNull();
    expect(win("F11")).toBe("fullscreen");
    expect(mac("Escape")).toBe("exit-fullscreen");
    expect(win("Escape")).toBe("exit-fullscreen");
  });

  it("ignores other keys", () => {
    expect(win("a")).toBeNull();
    expect(mac("0")).toBeNull();
  });
});
