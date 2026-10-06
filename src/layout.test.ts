import { describe, expect, it } from "vitest";
import { captureAnchor, currentPageIndex, restoreScrollTop, type PageBox } from "./layout";

const boxes: PageBox[] = [
  { top: 0, height: 1000 },
  { top: 1000, height: 500 },
  { top: 1500, height: 1500 },
];

describe("currentPageIndex", () => {
  it("returns the page that contains y", () => {
    expect(currentPageIndex(boxes, 0)).toBe(0);
    expect(currentPageIndex(boxes, 999)).toBe(0);
    expect(currentPageIndex(boxes, 1000)).toBe(1);
    expect(currentPageIndex(boxes, 2999)).toBe(2);
    expect(currentPageIndex(boxes, 99999)).toBe(2);
  });

  it("handles empty and negative input", () => {
    expect(currentPageIndex([], 10)).toBe(-1);
    expect(currentPageIndex(boxes, -50)).toBe(0);
  });
});

describe("zoom anchor", () => {
  it("keeps the same point of the current page at the viewport center", () => {
    // Viewport 800 high, scrolled so the center (y=1250) is half-way into page 1.
    const anchor = captureAnchor(boxes, 850, 800);
    expect(anchor).toEqual({ index: 1, fraction: 0.5 });
    // After zoom to 200%, every page is twice as tall.
    const zoomed = boxes.map((b) => ({ top: b.top * 2, height: b.height * 2 }));
    expect(restoreScrollTop(anchor!, zoomed, 800)).toBe(2000 + 500 - 400);
  });

  it("never returns a negative scroll position", () => {
    const anchor = captureAnchor(boxes, 0, 800)!;
    expect(restoreScrollTop(anchor, boxes, 800)).toBe(0);
  });

  it("returns null without pages", () => {
    expect(captureAnchor([], 0, 800)).toBeNull();
  });
});
