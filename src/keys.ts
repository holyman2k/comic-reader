export type KeyAction = "zoom-in" | "zoom-out" | "zoom-reset" | "fullscreen" | "exit-fullscreen";

export interface KeyInput {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  isMac: boolean;
}

export function keyAction({ key, ctrlKey, metaKey, isMac }: KeyInput): KeyAction | null {
  const mod = isMac ? metaKey && !ctrlKey : ctrlKey && !metaKey;
  if (mod && (key === "+" || key === "=")) return "zoom-in";
  if (mod && key === "-") return "zoom-out";
  if (mod && key === "0") return "zoom-reset";
  if (!isMac && key === "F11") return "fullscreen";
  if (isMac && ctrlKey && metaKey && key.toLowerCase() === "f") return "fullscreen";
  if (key === "Escape") return "exit-fullscreen";
  return null;
}
