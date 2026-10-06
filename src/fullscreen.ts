import { getCurrentWindow } from "@tauri-apps/api/window";

/** Mouse within this distance of the top edge shows the toolbar in full screen. */
const REVEAL_PX = 40;
/** Mouse below this distance hides it again (the gap avoids flicker). */
const HIDE_PX = 80;

export class Fullscreen {
  private active = false;

  constructor() {
    // Also catches full screen changes from the macOS green button and menu.
    void getCurrentWindow().onResized(() => void this.sync());
    document.addEventListener("mousemove", (e) => {
      if (!this.active) return;
      if (e.clientY <= REVEAL_PX) document.body.classList.add("reveal-toolbar");
      else if (e.clientY > HIDE_PX) document.body.classList.remove("reveal-toolbar");
    });
    void this.sync();
  }

  async toggle(): Promise<void> {
    await getCurrentWindow().setFullscreen(!this.active);
    await this.sync();
  }

  async exit(): Promise<void> {
    if (!this.active) return;
    await getCurrentWindow().setFullscreen(false);
    await this.sync();
  }

  private async sync(): Promise<void> {
    this.active = await getCurrentWindow().isFullscreen();
    document.body.classList.toggle("fullscreen", this.active);
    if (!this.active) document.body.classList.remove("reveal-toolbar");
  }
}
