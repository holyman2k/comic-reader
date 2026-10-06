import { convertFileSrc } from "@tauri-apps/api/core";
import { captureAnchor, currentPageIndex, restoreScrollTop, type Anchor, type PageBox } from "./layout";
import type { BookInfo } from "./types";

const UNKNOWN_RATIO = "2 / 3";
/** Pages within about two screen heights of the viewport stay loaded. */
const LOAD_MARGIN = "200% 0px";

export class Viewer {
  onPageChange: (current: number, total: number) => void = () => {};

  private readonly scroller: HTMLElement;
  private readonly column: HTMLElement;
  private readonly observer: IntersectionObserver;
  private pages: HTMLElement[] = [];
  private book: BookInfo | null = null;
  private frame = 0;
  private lastAnchor: Anchor | null = null;
  private lastWidth = 0;

  constructor(scroller: HTMLElement, column: HTMLElement) {
    this.scroller = scroller;
    this.column = column;
    this.observer = new IntersectionObserver((entries) => this.onIntersect(entries), {
      root: scroller,
      rootMargin: LOAD_MARGIN,
    });
    scroller.addEventListener("scroll", () => this.scheduleUpdate(), { passive: true });
    new ResizeObserver(() => this.onResize()).observe(scroller);
  }

  show(book: BookInfo): void {
    this.clear();
    this.book = book;
    const fragment = document.createDocumentFragment();
    book.pages.forEach((page, index) => {
      const el = document.createElement("div");
      el.className = "page";
      el.dataset.index = String(index);
      el.style.aspectRatio = page.width && page.height ? `${page.width} / ${page.height}` : UNKNOWN_RATIO;

      const img = document.createElement("img");
      img.alt = page.name;
      img.decoding = "async";
      img.draggable = false;
      img.addEventListener("load", () => {
        el.classList.remove("failed");
        if (!page.width || !page.height) {
          el.style.aspectRatio = `${img.naturalWidth} / ${img.naturalHeight}`;
        }
      });
      img.addEventListener("error", () => {
        if (!img.getAttribute("src")) return;
        el.classList.add("failed");
        el.dataset.message = `Page ${index + 1} could not be loaded (${page.name})`;
      });

      el.appendChild(img);
      fragment.appendChild(el);
      this.pages.push(el);
    });
    this.column.appendChild(fragment);
    this.pages.forEach((el) => this.observer.observe(el));
    this.scroller.scrollTo(0, 0);
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
  }

  setZoom(zoom: number): void {
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
    const book = this.book;
    if (!book) return;
    for (const entry of entries) {
      const el = entry.target as HTMLElement;
      const img = el.querySelector("img");
      if (!img) continue;
      if (entry.isIntersecting && !img.getAttribute("src")) {
        img.src = convertFileSrc(`${book.bookId}/${el.dataset.index}`, "comic");
      } else if (!entry.isIntersecting && img.getAttribute("src")) {
        img.removeAttribute("src"); // frees the decoded image
      }
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
    });
  }
}
