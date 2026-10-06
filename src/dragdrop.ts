import { getCurrentWebview } from "@tauri-apps/api/webview";

export async function setupDragDrop(onDrop: (path: string) => void): Promise<void> {
  const highlight = document.getElementById("drop-highlight");
  if (!highlight) throw new Error("Missing #drop-highlight");
  await getCurrentWebview().onDragDropEvent((event) => {
    const payload = event.payload;
    if (payload.type === "enter" || payload.type === "over") {
      highlight.hidden = false;
      return;
    }
    highlight.hidden = true;
    if (payload.type === "drop" && payload.paths.length > 0) onDrop(payload.paths[0]);
  });
}
