# Progress file failures

Status: resolved
Type: AFK
Blocked by: 01

## Parent

`.scratch/reading-progress/spec.md`

## What to build

Progress is a convenience and must never interrupt reading. If the progress file is corrupt, move it to `progress.json.bak` and start with an empty store. If a write fails (disk full, read-only), log it and keep reading. Neither case blocks opening a comic or shows an error to the reader.

## Acceptance criteria

- [ ] A corrupt progress file: the app opens comics normally, the bad file is kept as `progress.json.bak`, and a new file is written on the next save.
- [ ] A read-only data dir: comics open and scroll normally, and no error shows.
- [ ] Both cases have unit tests.
