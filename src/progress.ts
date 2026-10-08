import type { Resume } from "./types";

/** Pixels of slack for fractional scroll positions. */
const BOTTOM_EPSILON = 2;

/** True when the viewport shows the end of the content. */
export function atBottom(scrollTop: number, clientHeight: number, scrollHeight: number): boolean {
  return scrollTop + clientHeight >= scrollHeight - BOTTOM_EPSILON;
}

/** Page index to open at: the saved page, or the first page for a new or finished comic. */
export function resumeIndex(resume: Resume | null | undefined, pageCount: number): number {
  if (!resume || resume.finished) return 0;
  return Math.max(0, Math.min(resume.pageIndex, pageCount - 1));
}

export interface ProgressReport {
  pageIndex: number;
  finished: boolean;
}

/**
 * Decides when the viewer's position is worth reporting. The page the comic
 * opened on is the baseline: it is not reported until the reader moves.
 */
export class ProgressReporter {
  private active = false;
  private lastIndex = 0;
  private bottomSeen = false;

  /** Starts a comic once its restore is done. */
  begin(index: number, bottom: boolean): void {
    this.active = true;
    this.lastIndex = index;
    this.bottomSeen = bottom; // opening at the bottom is not reading to the end
  }

  end(): void {
    this.active = false;
  }

  update(index: number, bottom: boolean): ProgressReport | null {
    if (!this.active) return null;
    const reachedBottom = bottom && !this.bottomSeen;
    this.bottomSeen = bottom;
    if (index === this.lastIndex && !reachedBottom) return null;
    this.lastIndex = index;
    return { pageIndex: index, finished: bottom };
  }
}
