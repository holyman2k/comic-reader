export const ZOOM_MIN = 25;
export const ZOOM_MAX = 200;
export const ZOOM_STEP = 10;
export const ZOOM_DEFAULT = 100;

const STORAGE_KEY = "comic-reader.zoom";

export function clampZoom(zoom: number): number {
  if (!Number.isFinite(zoom)) return ZOOM_DEFAULT;
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Math.round(zoom)));
}

export const zoomIn = (zoom: number): number => clampZoom(zoom + ZOOM_STEP);
export const zoomOut = (zoom: number): number => clampZoom(zoom - ZOOM_STEP);

export function loadZoom(storage: Pick<Storage, "getItem"> | null): number {
  try {
    const raw = storage?.getItem(STORAGE_KEY);
    if (raw == null || raw === "") return ZOOM_DEFAULT;
    return clampZoom(Number(raw));
  } catch {
    return ZOOM_DEFAULT;
  }
}

export function saveZoom(storage: Pick<Storage, "setItem"> | null, zoom: number): void {
  try {
    storage?.setItem(STORAGE_KEY, String(zoom));
  } catch {
    // Storage blocked: the zoom is simply not remembered.
  }
}
