# Flush progress on comic switch, window close and quit

Status: ready-for-agent
Type: AFK
Blocked by: 01

## Parent

`.scratch/reading-progress/spec.md`

## What to build

Save progress immediately, without waiting for the debounce, when another comic opens, when the window closes, and when the app quits. On macOS the app stays alive after the last window closes, so window close must flush by itself.

## Acceptance criteria

- [ ] Change page in comic A, then open comic B within 1 s: A's last reached page is stored.
- [ ] Change page, then close the window within 1 s: the page is stored.
- [ ] Change page, then quit the app within 1 s: the page is stored.
- [ ] A flush with nothing changed writes nothing.
