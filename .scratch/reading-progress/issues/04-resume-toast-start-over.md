# Resume toast with Start over

Status: ready-for-agent
Type: AFK
Blocked by: 01

## Parent

`.scratch/reading-progress/spec.md`

## What to build

When a comic resumes, show a toast "Resumed at page N · Start over" for 8 s. Do not show it when the saved page is 1 or the comic was finished. "Start over" scrolls to page 1. The normal save then stores page 1. There is no special delete.

## Acceptance criteria

- [ ] Resume at page 12: the toast shows "Resumed at page 12" with a Start over action, and hides after 8 s.
- [ ] Start over scrolls to page 1 and the stored progress becomes page 1.
- [ ] No toast for a first open, a saved page 1, or a finished comic.
- [ ] Opening another comic hides the toast.
