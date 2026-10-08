# Path fallback for edited comics

Status: resolved
Type: AFK
Blocked by: 01

## Parent

`.scratch/reading-progress/spec.md`

## What to build

Adding, removing or editing a page changes a comic's fingerprint. When no stored fingerprint matches, look for an entry with the same last-known source path. Reuse it only if its saved page name exists in the new page list. Then update the entry's fingerprint. A reused path that no longer holds the saved page is treated as a new comic.

## Acceptance criteria

- [ ] Edit one page of a comic with progress, reopen from the same path: progress is kept and the entry has the new fingerprint.
- [ ] Replace the file at the path with a different comic that lacks the saved page name: no resume.
- [ ] A fingerprint match always wins over a path match.
- [ ] Matching rules have unit tests.
