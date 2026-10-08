export function displayName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function openingLabel(path: string): string {
  const name = displayName(path);
  return /\.(rar|cbr|tgz|tar\.gz)$/i.test(name) ? `Extracting ${name}…` : `Opening ${name}…`;
}

export function formatCounter(current: number, total: number): string {
  return total > 0 ? `${current} / ${total}` : "";
}

export function resumeToastText(page: number): string {
  return `Back at page ${page}`;
}

export function finishedTitle(title: string): string {
  return `Finished ${title}`;
}

export function finishedDetail(pageCount: number): string {
  const pages = pageCount === 1 ? "1 page" : `${pageCount} pages`;
  return `${pages}. Next time it opens at page 1.`;
}

/** Shortcut labels as the platform writes them. */
export function shortcutLabel(key: string, shift: boolean, isMac: boolean): string {
  if (isMac) return `${shift ? "⇧" : ""}⌘${key}`;
  return `Ctrl+${shift ? "Shift+" : ""}${key}`;
}
