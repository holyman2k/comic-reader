export type KeyAction =
  | "zoom-in"
  | "zoom-out"
  | "zoom-reset"
  | "open-file"
  | "open-folder"
  | "fullscreen"
  | "exit-fullscreen";

export interface KeyInput {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  isMac: boolean;
}

export function keyAction({ key, ctrlKey, metaKey, shiftKey, isMac }: KeyInput): KeyAction | null {
  const mod = isMac ? metaKey && !ctrlKey : ctrlKey && !metaKey;
  if (mod && (key === "+" || key === "=")) return "zoom-in";
  if (mod && key === "-") return "zoom-out";
  if (mod && key === "0") return "zoom-reset";
  if (mod && key.toLowerCase() === "o") return shiftKey ? "open-folder" : "open-file";
  if (!isMac && key === "F11") return "fullscreen";
  if (isMac && ctrlKey && metaKey && key.toLowerCase() === "f") return "fullscreen";
  if (key === "Escape") return "exit-fullscreen";
  return null;
}
