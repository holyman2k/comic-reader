import { convertFileSrc } from "@tauri-apps/api/core";
import { atBottom } from "./progress";
import { captureAnchor, currentPageIndex, restoreScrollTop, type Anchor, type PageBox } from "./layout";
import type { BookInfo } from "./types";

const UNKNOWN_RATIO = "2 / 3";
/** Pages within about two screen heights of the viewport stay loaded. */
const LOAD_MARGIN = "200% 0px";

export class Viewer {
  onPageChange: (current: number, total: number) => void = () => {};
  /** The restore is done: the page at the viewport top, and whether the bottom is showing. */
  onSettled: (reachedIndex: number, bottom: boolean) => void = () => {};
  /** After `onSettled`, on every update: the page at the viewport top. */
  onReach: (reachedIndex: number, bottom: boolean) => void = () => {};

  private readonly scroller: HTMLElement;
  private readonly column: HTMLElement;
  private readonly observer: IntersectionObserver;
  private pages: HTMLElement[] = [];
  private book: BookInfo | null = null;
  private frame = 0;
  private lastAnchor: Anchor | null = null;
  private lastWidth = 0;
  private settled = false;
  /** Page whose top is kept at the viewport top while pages above it load. */
  private holdIndex: number | null = null;
  private retries = 0;

  constructor(scroller: HTMLElement, column: HTMLElement) {
    this.scroller = scroller;
    this.column = column;
    this.observer = new IntersectionObserver((entries) => this.onIntersect(entries), {
      root: scroller,
      rootMargin: LOAD_MARGIN,
    });
    scroller.addEventListener("scroll", () => this.scheduleUpdate(), { passive: true });
    new ResizeObserver(() => this.onResize()).observe(scroller);
    // The first reader input ends the position correction.
    for (const type of ["wheel", "touchstart", "pointerdown"]) {
      scroller.addEventListener(type, () => this.releaseHold(), { passive: true });
    }
    document.addEventListener("keydown", () => this.releaseHold());
  }

  /** Opens `book` with the page at `startIndex` at the viewport top. */
  show(book: BookInfo, startIndex = 0): void {
    this.clear();
    this.book = book;
    const fragment = document.createDocumentFragment();
    book.pages.forEach((page, index) => {
      const el = document.createElement("div");
      el.className = "page";
      el.dataset.index = String(index);
      el.dataset.number = String(index + 1);
      el.style.aspectRatio = page.width && page.height ? `${page.width} / ${page.height}` : UNKNOWN_RATIO;

      const img = document.createElement("img");
      img.alt = page.name;
      img.decoding = "async";
      img.draggable = false;
      img.addEventListener("load", () => {
        el.classList.remove("loading", "failed");
        el.classList.add("loaded");
        el.querySelector(".page-error")?.remove();
        if (!page.width || !page.height) {
          el.style.aspectRatio = `${img.naturalWidth} / ${img.naturalHeight}`;
        }
        this.holdPosition(index);
      });
      img.addEventListener("error", () => {
        if (!img.getAttribute("src")) return;
        el.classList.remove("loading");
        el.classList.add("failed");
        if (!el.querySelector(".page-error")) el.appendChild(this.errorCard(el, img, index, page.name));
      });

      el.appendChild(img);
      fragment.appendChild(el);
      this.pages.push(el);
    });
    this.column.appendChild(fragment);
    this.pages.forEach((el) => this.observer.observe(el));
    this.scrollToPage(startIndex);
    this.holdIndex = startIndex > 0 ? startIndex : null;
    this.scroller.focus({ preventScroll: true });
    this.lastWidth = this.scroller.clientWidth;
    this.scheduleUpdate();
  }

  clear(): void {
    this.observer.disconnect();
    this.pages.forEach((el) => el.querySelector("img")?.removeAttribute("src"));
    this.column.replaceChildren();
    this.pages = [];
    this.book = null;
    this.lastAnchor = null;
    this.lastWidth = this.scroller.clientWidth;
    this.settled = false;
    this.holdIndex = null;
  }

  /** The card a failed page shows in place of its image. */
  private errorCard(el: HTMLElement, img: HTMLImageElement, index: number, name: string): HTMLElement {
    const card = document.createElement("div");
    card.className = "page-error";
    const icon = document.createElement("span");
    icon.className = "page-error-icon";
    icon.textContent = "!";
    const title = document.createElement("strong");
    title.textContent = `Page ${index + 1} could not be loaded`;
    const detail = document.createElement("span");
    detail.textContent = name;
    const retry = document.createElement("button");
    retry.textContent = "Retry";
    // Keep focus on the page scroller so Space and Enter do not re-click the button.
    retry.addEventListener("mousedown", (e) => e.preventDefault());
    retry.addEventListener("click", () => this.load(el, img, index, ++this.retries));
    card.append(icon, title, detail, retry);
    return card;
  }

  /** Starts loading page `index`. A retry count gives a new URL, so a failed response is not reused. */
  private load(el: HTMLElement, img: HTMLImageElement, index: number, retry = 0): void {
    const book = this.book;
    if (!book) return;
    el.querySelector(".page-error")?.remove();
    el.classList.remove("failed");
    el.classList.add("loading");
    const url = convertFileSrc(`${book.bookId}/${index}`, "comic");
    img.src = retry ? `${url}?retry=${retry}` : url;
  }

  /** Scrolls so the top of page `index` meets the top of the viewport. */
  scrollToPage(index: number): void {
    this.holdIndex = null;
    const el = this.pages[index];
    this.scroller.scrollTop = el ? el.offsetTop : 0;
  }

  /** Pages above the held page changed height: put its top back at the viewport top. */
  private holdPosition(changed: number): void {
    const held = this.holdIndex;
    if (held === null || changed >= held) return;
    const el = this.pages[held];
    if (el) this.scroller.scrollTop = el.offsetTop;
  }

  private releaseHold(): void {
    this.holdIndex = null;
  }

  setZoom(zoom: number): void {
    this.holdIndex = null;
    const anchor = captureAnchor(this.boxes(), this.scroller.scrollTop, this.scroller.clientHeight);
    document.documentElement.style.setProperty("--zoom", String(zoom));
    if (anchor) {
      this.scroller.scrollTop = restoreScrollTop(anchor, this.boxes(), this.scroller.clientHeight);
    }
    this.lastAnchor = captureAnchor(this.boxes(), this.scroller.scrollTop, this.scroller.clientHeight);
    this.lastWidth = this.scroller.clientWidth;
    this.scheduleUpdate();
  }

  /** Page heights follow the column width, so keep the same page at the center. */
  private onResize(): void {
    const width = this.scroller.clientWidth;
    if (width !== this.lastWidth) {
      this.lastWidth = width;
      this.holdIndex = null;
      if (this.lastAnchor) {
        this.scroller.scrollTop = restoreScrollTop(this.lastAnchor, this.boxes(), this.scroller.clientHeight);
      }
    }
    this.scheduleUpdate();
  }

  private boxes(): PageBox[] {
    return this.pages.map((el) => ({ top: el.offsetTop, height: el.offsetHeight }));
  }

  private onIntersect(entries: IntersectionObserverEntry[]): void {
    if (!this.book) return;
    for (const entry of entries) {
      const el = entry.target as HTMLElement;
      const img = el.querySelector("img");
      if (!img) continue;
      if (entry.isIntersecting && !img.getAttribute("src")) {
        this.load(el, img, Number(el.dataset.index));
      } else if (!entry.isIntersecting && img.getAttribute("src")) {
        img.removeAttribute("src"); // frees the decoded image
        el.classList.remove("loading", "loaded");
      }
    }
  }

  /** The reached page is the one at the top edge of the viewport. */
  private reportReach(): void {
    if (this.pages.length === 0) return;
    const { scrollTop, clientHeight } = this.scroller;
    const boxes = this.boxes();
    // One pixel of slack: a restored scrollTop can round just below the page top.
    const reached = currentPageIndex(boxes, scrollTop + 1);
    // The end card sits below the last page, so the bottom is the last page's bottom.
    const last = boxes[boxes.length - 1];
    const bottom = atBottom(scrollTop, clientHeight, last.top + last.height);
    if (this.settled) {
      this.onReach(reached, bottom);
    } else {
      this.settled = true;
      this.onSettled(reached, bottom);
    }
  }

  private scheduleUpdate(): void {
    if (this.frame) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      // Skip the capture while a width change is pending: the layout is new, scrollTop is old.
      if (this.scroller.clientWidth === this.lastWidth) {
        this.lastAnchor = captureAnchor(this.boxes(), this.scroller.scrollTop, this.scroller.clientHeight);
      }
      const total = this.pages.length;
      const center = this.scroller.scrollTop + this.scroller.clientHeight / 2;
      const index = currentPageIndex(this.boxes(), center);
      this.onPageChange(index + 1, total);
      this.reportReach();
    });
  }
}
