export interface PageBox {
  top: number;
  height: number;
}

/** A point inside a page: which page, and how far down it (0..1). */
export interface Anchor {
  index: number;
  fraction: number;
}

/** Index of the last page whose top is at or above `y`; -1 when there are no pages. */
export function currentPageIndex(boxes: PageBox[], y: number): number {
  if (boxes.length === 0) return -1;
  let lo = 0;
  let hi = boxes.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (boxes[mid].top <= y) lo = mid;
    else hi = mid - 1;
  }
  return lo;
}

export function captureAnchor(boxes: PageBox[], scrollTop: number, viewportHeight: number): Anchor | null {
  const center = scrollTop + viewportHeight / 2;
  const index = currentPageIndex(boxes, center);
  if (index < 0) return null;
  const box = boxes[index];
  const fraction = box.height > 0 ? Math.min(Math.max((center - box.top) / box.height, 0), 1) : 0;
  return { index, fraction };
}

export function restoreScrollTop(anchor: Anchor, boxes: PageBox[], viewportHeight: number): number {
  const box = boxes[anchor.index];
  if (!box) return 0;
  return Math.max(0, box.top + anchor.fraction * box.height - viewportHeight / 2);
}
