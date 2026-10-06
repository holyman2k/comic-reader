import { describe, expect, it } from "vitest";
import { clampZoom, loadZoom, saveZoom, zoomIn, zoomOut, ZOOM_DEFAULT } from "./zoom";

describe("zoom", () => {
  it("clamps to 25..200 and rounds", () => {
    expect(clampZoom(10)).toBe(25);
    expect(clampZoom(250)).toBe(200);
    expect(clampZoom(99.6)).toBe(100);
    expect(clampZoom(Number.NaN)).toBe(ZOOM_DEFAULT);
  });

  it("steps by 10 within limits", () => {
    expect(zoomIn(100)).toBe(110);
    expect(zoomOut(100)).toBe(90);
    expect(zoomIn(195)).toBe(200);
    expect(zoomIn(200)).toBe(200);
    expect(zoomOut(30)).toBe(25);
    expect(zoomOut(25)).toBe(25);
  });

  it("loads saved zoom or falls back to default", () => {
    const store = new Map<string, string>();
    const storage = {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
    };
    expect(loadZoom(storage)).toBe(ZOOM_DEFAULT);
    saveZoom(storage, 140);
    expect(loadZoom(storage)).toBe(140);
    store.set("comic-reader.zoom", "garbage");
    expect(loadZoom(storage)).toBe(ZOOM_DEFAULT);
    store.set("comic-reader.zoom", "");
    expect(loadZoom(storage)).toBe(ZOOM_DEFAULT);
    expect(loadZoom(null)).toBe(ZOOM_DEFAULT);
  });

  it("survives storage that throws", () => {
    const broken = {
      getItem: () => { throw new Error("blocked"); },
      setItem: () => { throw new Error("blocked"); },
    };
    expect(loadZoom(broken)).toBe(ZOOM_DEFAULT);
    expect(() => saveZoom(broken, 120)).not.toThrow();
  });
});
