import { describe, expect, it } from "vitest";
import { atBottom, ProgressReporter, resumeIndex } from "./progress";

describe("atBottom", () => {
  it("is true when the viewport reaches the end of the content", () => {
    expect(atBottom(1000, 800, 1800)).toBe(true);
  });

  it("allows a small rounding gap", () => {
    expect(atBottom(999.5, 800, 1800)).toBe(true);
    expect(atBottom(997, 800, 1800)).toBe(false);
  });

  it("is true for content shorter than the viewport", () => {
    expect(atBottom(0, 800, 300)).toBe(true);
  });

  it("is false in the middle", () => {
    expect(atBottom(500, 800, 1800)).toBe(false);
  });
});

describe("resumeIndex", () => {
  it("opens an unknown comic at page 1", () => {
    expect(resumeIndex(undefined, 10)).toBe(0);
    expect(resumeIndex(null, 10)).toBe(0);
  });

  it("opens a finished comic at page 1", () => {
    expect(resumeIndex({ pageIndex: 7, finished: true }, 10)).toBe(0);
  });

  it("opens at the saved page, kept inside the page list", () => {
    expect(resumeIndex({ pageIndex: 7, finished: false }, 10)).toBe(7);
    expect(resumeIndex({ pageIndex: 50, finished: false }, 10)).toBe(9);
  });
});

describe("ProgressReporter", () => {
  it("reports nothing before the restore is done", () => {
    const r = new ProgressReporter();
    expect(r.update(3, false)).toBeNull();
  });

  it("reports nothing while the reader stays on the starting page", () => {
    const r = new ProgressReporter();
    r.begin(0, false);
    expect(r.update(0, false)).toBeNull();
    expect(r.update(0, false)).toBeNull();
  });

  it("reports a changed reached page", () => {
    const r = new ProgressReporter();
    r.begin(0, false);
    expect(r.update(1, false)).toEqual({ pageIndex: 1, finished: false });
    expect(r.update(1, false)).toBeNull();
    expect(r.update(0, false)).toEqual({ pageIndex: 0, finished: false });
  });

  it("does not report the restored page again", () => {
    const r = new ProgressReporter();
    r.begin(12, false);
    expect(r.update(12, false)).toBeNull();
  });

  it("reports finished once when the bottom is reached", () => {
    const r = new ProgressReporter();
    r.begin(0, false);
    expect(r.update(4, true)).toEqual({ pageIndex: 4, finished: true });
    expect(r.update(4, true)).toBeNull();
  });

  it("marks finished when the bottom is reached on the same page", () => {
    const r = new ProgressReporter();
    r.begin(4, false);
    expect(r.update(4, true)).toEqual({ pageIndex: 4, finished: true });
  });

  it("does not mark finished when the comic opens already at the bottom", () => {
    const r = new ProgressReporter();
    r.begin(0, true); // a one-page comic that fits the window
    expect(r.update(0, true)).toBeNull();
  });

  it("reports finished again after leaving and returning to the bottom", () => {
    const r = new ProgressReporter();
    r.begin(0, true);
    expect(r.update(0, false)).toBeNull();
    expect(r.update(0, true)).toEqual({ pageIndex: 0, finished: true });
  });

  it("clears finished when the reader moves to an earlier page", () => {
    const r = new ProgressReporter();
    r.begin(0, false);
    r.update(4, true);
    expect(r.update(2, false)).toEqual({ pageIndex: 2, finished: false });
  });

  it("stops after end() until the next begin()", () => {
    const r = new ProgressReporter();
    r.begin(0, false);
    r.end();
    expect(r.update(3, false)).toBeNull();
    r.begin(0, false);
    expect(r.update(3, false)).toEqual({ pageIndex: 3, finished: false });
  });
});
