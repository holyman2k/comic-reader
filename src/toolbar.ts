export interface ToolbarHandlers {
  openFile(): void;
  openFolder(): void;
  zoomIn(): void;
  zoomOut(): void;
  toggleFullscreen(): void;
}

function byId(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`Missing #${id}`);
  return el;
}

function bind(selector: string, handler: () => void): void {
  document.querySelectorAll<HTMLElement>(selector).forEach((el) => {
    // Keep focus on the page scroller so Space and Enter do not re-click the button.
    el.addEventListener("mousedown", (e) => e.preventDefault());
    el.addEventListener("click", handler);
  });
}

export function setupToolbar(h: ToolbarHandlers): void {
  bind("#open-file, [data-open=file]", h.openFile);
  bind("#open-folder, [data-open=folder]", h.openFolder);
  bind("#zoom-in", h.zoomIn);
  bind("#zoom-out", h.zoomOut);
  bind("#fullscreen", h.toggleFullscreen);
}

export function setZoomLabel(zoom: number): void {
  byId("zoom-label").textContent = `${zoom}%`;
}

export function setCounter(text: string): void {
  byId("counter").textContent = text;
  byId("counter-overlay").textContent = text;
}
