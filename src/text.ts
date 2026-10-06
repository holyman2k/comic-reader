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
