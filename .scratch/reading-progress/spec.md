# Reading progress

Status: ready-for-agent

Remember where the reader stopped in each comic, and resume there when the same comic opens again. Terms follow `CONTEXT.md`. Identity decision: `docs/adr/0001-comic-identity-by-page-list-fingerprint.md`.

## Scope

In: saving and resuming progress for archive and folder sources.
Out: a library or "recent / in progress" list, zoom or scroll offset within a page, syncing between devices.

## Identity

- A comic is identified by its **fingerprint**: a hash of the sorted page list, one `name + byte size` per page.
- Remove the common leading directory from page names before hashing, so an archive (`Comic/001.jpg`) and the same folder unzipped (`001.jpg`) match.
- Same fingerprint means shared progress, even for copies in two places.
- Fallback: if no fingerprint matches, but a stored entry has the same last-known source path and its saved page name exists in the new page list, reuse that entry and update its fingerprint.
- `PageSource` (`src-tauri/src/source/mod.rs`) needs a `size(path)` method. Zip reads it from the central directory. Folder and extracted rar/tar.gz use `stat`. No full read of any source.
- `BookInfo` carries the fingerprint to the frontend.

## Stored data

JSON file in the app data dir, written by Rust, with a top-level `version` field. One entry per comic:

- fingerprint
- last source path
- reached page name and index
- finished flag
- last-opened time

Entries are kept forever. An entry is created only when the reader passes page 1 or finishes, or when one already exists.

## Reached page and finished

- **Reached page** = the page at the top edge of the viewport. The toolbar counter keeps its viewport-center rule.
- **Finished** = the reader scrolled to the bottom of the last page (`scrollTop + clientHeight >= scrollHeight - epsilon`). The center rule cannot be used: a short last page may never reach the center.
- Reaching any page after reopening a finished comic clears the finished flag. The latest position wins.

## Resume

- Known comic, not finished: scroll so the saved page top is at the viewport top.
- Finished comic: open at page 1.
- Show a toast "Resumed at page N · Start over" for 8 s. No toast when the saved page is 1 or the comic was finished.
- "Start over" scrolls to page 1. The normal save then stores page 1. No special delete.
- Pages with unknown size use a 2/3 placeholder ratio, so heights above the target can shift. After restore, re-apply the anchor when pages near the target finish loading. Stop at the first wheel, key or touch input.

## Saving

- Debounced: 1 s after the reached page stops changing.
- Immediate flush when another comic opens, the window closes, or the app quits. On macOS the app stays alive after the last window closes, so window close must flush.
- No saving for a newly opened comic until its restore is done. `Viewer.show()` scrolls to 0 and reports page 1, which must not overwrite stored progress.
- Each save reads the file, changes only this comic's entry, and writes atomically (temp file + rename). Two instances may race, but one never wipes the other's entries.

## Failures

- Corrupt file: move it to `progress.json.bak` and start empty.
- Write failure (disk full, read-only): log it and keep reading.
- Neither blocks opening a comic. No error shown to the reader.

## Acceptance

- Close at page 12, reopen the same archive: it opens at page 12 with the toast.
- Move or rename the archive or folder: progress is kept.
- Unzip an archive to a folder: opening the folder resumes the archive's progress.
- Scroll to the bottom, reopen: starts at page 1, no toast.
- Open a comic and close without scrolling: no entry stored.
- Zoomed out so the last page is shorter than half the viewport: bottom reached still marks finished.
- Reopen the same comic repeatedly at a short page: the reached page does not drift forward.
- Open comic B right after comic A: A's progress is saved, B's progress is not overwritten by page 1.
- Corrupt progress file: the app opens comics normally and starts a new file.
