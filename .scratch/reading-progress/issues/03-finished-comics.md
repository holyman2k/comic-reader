# Finished comics

Status: resolved
Type: AFK
Blocked by: 01

## Parent

`.scratch/reading-progress/spec.md`

## What to build

Mark a comic finished when the reader scrolls to the bottom of its last page. Do not use the center rule: a short last page may never reach the viewport center. A finished comic opens at page 1. If the reader then reaches a later page and leaves, the latest position is stored and the finished state is cleared.

## Acceptance criteria

- [ ] Scroll to the bottom, reopen: the comic opens at page 1.
- [ ] Zoomed out so the last page is shorter than half the viewport: reaching the bottom still marks the comic finished.
- [ ] Reopen a finished comic, scroll to page 3, close: reopening resumes at page 3 and the comic is not finished.
- [ ] Bottom detection has unit tests.
