import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { Fullscreen } from "./fullscreen";
import { keyAction } from "./keys";
import { formatCounter, openingLabel } from "./text";
import { setCounter, setupToolbar, setZoomLabel } from "./toolbar";
import { SUPERSEDED, type BookInfo } from "./types";
import { Viewer } from "./viewer";
import { loadZoom, saveZoom, zoomIn, zoomOut, ZOOM_DEFAULT } from "./zoom";

const ARCHIVE_EXTENSIONS = ["zip", "cbz", "rar", "cbr", "gz", "tgz"];
const ERROR_VISIBLE_MS = 6000;
const isMac = navigator.userAgent.includes("Mac");

function byId(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`Missing #${id}`);
  return el;
}

function safeLocalStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

const storage = safeLocalStorage();
const viewer = new Viewer(byId("scroller"), byId("pages"));
const fullscreen = new Fullscreen();
const status = byId("status");
let zoom = loadZoom(storage);
let statusTimer = 0;

function showStatus(text: string, isError = false): void {
  window.clearTimeout(statusTimer);
  status.textContent = text;
  status.classList.toggle("error", isError);
  status.hidden = false;
  if (isError) statusTimer = window.setTimeout(hideStatus, ERROR_VISIBLE_MS);
}

function hideStatus(): void {
  window.clearTimeout(statusTimer);
  status.hidden = true;
}

status.addEventListener("click", () => {
  if (status.classList.contains("error")) hideStatus();
});

export async function openPath(path: string): Promise<void> {
  showStatus(openingLabel(path));
  try {
    const book = await invoke<BookInfo>("open_book", { path });
    hideStatus();
    viewer.show(book);
    document.body.classList.add("has-book");
    await getCurrentWindow().setTitle(`${book.title} — Comic Reader`);
  } catch (err) {
    if (err === SUPERSEDED) return; // a newer open owns the status line
    showStatus(String(err), true);
  }
}

async function chooseFile(): Promise<void> {
  const path = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Comics", extensions: ARCHIVE_EXTENSIONS }],
  });
  if (typeof path === "string") await openPath(path);
}

async function chooseFolder(): Promise<void> {
  const path = await open({ multiple: false, directory: true });
  if (typeof path === "string") await openPath(path);
}

function applyZoom(next: number): void {
  zoom = next;
  viewer.setZoom(zoom);
  setZoomLabel(zoom);
  saveZoom(storage, zoom);
}

setupToolbar({
  openFile: () => void chooseFile(),
  openFolder: () => void chooseFolder(),
  zoomIn: () => applyZoom(zoomIn(zoom)),
  zoomOut: () => applyZoom(zoomOut(zoom)),
  toggleFullscreen: () => void fullscreen.toggle(),
});

viewer.onPageChange = (current, total) => setCounter(formatCounter(current, total));

document.addEventListener("keydown", (e) => {
  const action = keyAction({ key: e.key, ctrlKey: e.ctrlKey, metaKey: e.metaKey, isMac });
  if (!action) return;
  e.preventDefault();
  switch (action) {
    case "zoom-in":
      applyZoom(zoomIn(zoom));
      break;
    case "zoom-out":
      applyZoom(zoomOut(zoom));
      break;
    case "zoom-reset":
      applyZoom(ZOOM_DEFAULT);
      break;
    case "fullscreen":
      void fullscreen.toggle();
      break;
    case "exit-fullscreen":
      void fullscreen.exit();
      break;
  }
});

applyZoom(zoom);

async function openPending(): Promise<void> {
  const path = await invoke<string | null>("take_pending_open");
  if (path) await openPath(path);
}

// A Dock drop can arrive before this script runs (stored by the backend) or
// later (signalled by the event). Listen first, then check the store.
void listen("open-path", () => void openPending()).then(openPending);
