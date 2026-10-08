import { describe, expect, it } from "vitest";
import { keyAction } from "./keys";

type Mods = { ctrl?: boolean; meta?: boolean; shift?: boolean };
const press = (isMac: boolean) => (key: string, mods: Mods = {}) =>
  keyAction({ key, ctrlKey: !!mods.ctrl, metaKey: !!mods.meta, shiftKey: !!mods.shift, isMac });
const mac = press(true);
const win = press(false);

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

  it("maps open shortcuts, with Shift for a folder", () => {
    expect(mac("o", { meta: true })).toBe("open-file");
    expect(mac("o", { meta: true, shift: true })).toBe("open-folder");
    expect(mac("O", { meta: true, shift: true })).toBe("open-folder");
    expect(mac("o", { ctrl: true })).toBeNull();
    expect(win("o", { ctrl: true })).toBe("open-file");
    expect(win("O", { ctrl: true, shift: true })).toBe("open-folder");
    expect(win("o", { meta: true })).toBeNull();
    expect(win("o")).toBeNull();
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
