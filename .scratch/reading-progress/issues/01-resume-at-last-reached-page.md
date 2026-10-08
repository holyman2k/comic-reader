# Resume at last reached page

Status: ready-for-agent
Type: AFK
Blocked by: None

## Parent

`.scratch/reading-progress/spec.md`. Domain terms: `CONTEXT.md`. Identity decision: `docs/adr/0001-comic-identity-by-page-list-fingerprint.md`.

## What to build

The thin end-to-end path for reading progress. A comic gets a fingerprint when it opens. The viewer reports the reached page (the page at the viewport top edge). Progress is saved to a versioned JSON file in the app data dir. When a comic with stored progress opens again, the viewer scrolls to the saved page.

- The fingerprint is built from the sorted page names and byte sizes, with the common leading directory removed. An archive and the same folder unzipped get the same fingerprint. Page sizes come from the source index or `stat`, never from reading whole files.
- Each save reads the file, changes only this comic's entry, and writes atomically.
- Saves are debounced 1 s after the reached page stops changing.
- No entry is created until the reader passes page 1.
- No save happens for a newly opened comic until its restore is done, so page 1 never overwrites stored progress.
- The toolbar page counter keeps its current viewport-center rule.

## Acceptance criteria

- [ ] Close at page 12, reopen the same archive: it opens at page 12.
- [ ] Move or rename the archive or folder: progress is kept.
- [ ] Unzip an archive to a folder: opening the folder resumes the archive's progress.
- [ ] Reopen the same comic repeatedly at a short page: the reached page does not drift forward.
- [ ] Open a comic and close without scrolling: no entry is stored.
- [ ] Open comic B right after comic A: B's progress is not overwritten by page 1.
- [ ] Fingerprint and store logic have unit tests.
