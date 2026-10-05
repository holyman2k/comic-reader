# Comic Reader — Design Spec

Date: 2026-10-06
Status: Draft, awaiting review

## 1. Goal

A desktop app for macOS and Windows that reads comics and browses image sets.
It opens a folder or an archive, sorts the pages in natural order, and shows
all pages in one vertical column that scales to the window width.

## 2. Requirements

### 2.1 Sources

| Source | Extensions |
|---|---|
| Folder (including subfolders) | — |
| Zip | `.zip`, `.cbz` |
| RAR | `.rar`, `.cbr` |
| Gzip tarball | `.tar.gz`, `.tgz` |

- The source type is detected by magic bytes first and by extension second.
  A zip file named `.cbr` opens as zip.
- A plain `.gz` that holds a single file is not supported. Gzip is supported
  only as a tarball container.
- The app is read-only. It never edits, converts, or creates archives.
- Password-protected archives are not supported. They produce an error message.

### 2.2 Images

- Formats: JPEG, PNG, GIF, WebP, AVIF, BMP. Extensions matched
  case-insensitively: `jpg jpeg png gif webp avif bmp`.
- The web view decodes all images. The backend never decodes or converts
  image data.
- Excluded entries: names that start with `.`, and anything under `__MACOSX/`.

### 2.3 Sort order (natural sort)

Numbers are read from the file path only. The app does not use OCR.

1. Compare the path one folder level at a time. All of `ch2/` sorts before
   `ch10/`.
2. Split each name into digit runs (ASCII `0-9`) and text runs.
3. Text vs text: case-insensitive comparison.
4. Number vs number: compare by numeric value. Compare the digit strings
   (strip leading zeros, then compare length, then digits) so that long
   numbers never overflow.
5. Number vs text at the same position: the number comes first.
6. Tie-breaks, in order: fewer leading zeros first (`1` before `01`), then a
   byte-by-byte comparison of the original name. The order is total and
   deterministic.
7. `.`, `-`, and `,` are text. `1.5` is not a decimal and `-3` is not a
   negative number.

| Input | Expected order |
|---|---|
| `page 10`, `page 2`, `page 1` | `page 1`, `page 2`, `page 10` |
| `p010`, `p9`, `p1` | `p1`, `p9`, `p010` |
| `Ch2/01`, `ch10/01`, `ch2/02` | `Ch2/01`, `ch2/02`, `ch10/01` |
| `vol1 p2`, `vol1 p10`, `vol2 p1` | `vol1 p2`, `vol1 p10`, `vol2 p1` |

### 2.4 Ways to open a book

| Method | macOS | Windows |
|---|---|---|
| "Open File" and "Open Folder" dialogs | Yes | Yes |
| Drag and drop onto the app window | Yes | Yes |
| Drag and drop onto the Dock icon | Yes | — |
| Finder "Open With" | Yes | — |
| Command-line argument / drop onto `.exe` | — | No (out of scope) |

Two Open buttons are necessary because the Windows dialog cannot select
files and folders in one dialog.

### 2.5 Viewer features (version 1)

- Vertical column of pages, centered, no gap between pages, dark background.
- Zoom.
- Full screen.
- Page counter.

## 3. Tech stack

- **Tauri 2**: Rust backend, web frontend.
- **Frontend**: plain TypeScript and Vite. No UI framework.
- **Rust crates**: `zip`, `unrar`, `tar`, `flate2`, `imagesize`, `tempfile`.
- **Tauri plugins**: `tauri-plugin-dialog`.
- **Tests**: `cargo test` for Rust, Vitest for frontend logic.
- **Toolchain**: Rust stable, Node LTS. The `unrar` crate compiles C++ code:
  macOS needs Xcode Command Line Tools, Windows needs MSVC Build Tools.
- **RAR test fixtures** are created with the `rar` CLI, installed on macOS by
  `brew install --cask rar`.

Rejected alternatives: Avalonia/.NET and Flutter (user prefers Rust + web),
Swift (SwiftUI does not run on Windows).

## 4. Architecture

### 4.1 Page delivery

Pages are served on demand through a custom URI scheme:
`comic://localhost/{book_id}/{index}`.

- Each `<img>` points to this URL. The browser requests a page only when it
  comes near the viewport, and Rust reads only that entry.
- Folder and zip sources are read in random order directly.
- RAR and tar.gz cannot be read in random order efficiently (solid RAR, gzip
  stream). These sources are extracted once to a temporary folder, then
  served like a folder.

Rejected alternatives: extracting every source to disk first (slow start,
disk use, crash cleanup), and sending bytes through IPC (serialization
overhead, manual blob URL management).

### 4.2 Rust modules (`src-tauri/src/`)

| Module | Responsibility |
|---|---|
| `natural_sort` | `natural_cmp(a, b) -> Ordering`. Pure function, no I/O. |
| `source` | `PageSource` trait: `list() -> Vec<String>`, `read(path) -> Vec<u8>`. Type detection. |
| `source::FolderSource` | Reads files from a directory tree. |
| `source::ZipSource` | Random access to zip entries. |
| `source::ExtractedSource` | Extracts RAR / tar.gz to a temporary folder, then delegates to `FolderSource`. Deletes the folder on drop. |
| `book` | `open(path) -> Book`: filter image entries, sort with `natural_cmp`, read width × height from each header. |
| `protocol` | Handles `comic://`. Returns page bytes with the correct `Content-Type`. |
| `commands` | `open_book(path) -> BookInfo` and `take_pending_open() -> Option<String>`. |
| `launch_open` | macOS only. Handles `RunEvent::Opened { urls }` (Dock drop, Finder "Open With"). |

`BookInfo`:

```
{ book_id: u64, title: String, pages: [{ name: String, width: u32?, height: u32? }] }
```

- `book_id` increases on each open, so the browser cache never shows pages
  from a previous book.
- Width and height are absent when the header cannot be read.

### 4.3 Frontend modules (`src/`)

| Module | Responsibility |
|---|---|
| `viewer.ts` | Builds one placeholder per page with CSS `aspect-ratio`. An `IntersectionObserver` sets `src` when a page is within about 2 screen heights of the viewport and removes `src` when it is farther away. |
| `toolbar.ts` | Open File, Open Folder, zoom controls, full screen button, page counter. |
| `dragdrop.ts` | Listens to Tauri drag-drop events and calls `open_book`. Shows a drop highlight. |
| Pure logic modules | Zoom limits and steps, scroll anchor on zoom change, current page calculation. Kept free of DOM access so Vitest can test them. |

### 4.4 Launch-time open (macOS)

- File associations are declared only in `tauri.macos.conf.json`
  (`bundle.fileAssociations`) for `.zip`, `.cbz`, `.rar`, `.cbr`, `.tar.gz`,
  `.tgz`, and folders (`public.folder`). Role: Viewer. Rank: Alternate. The
  app appears in "Open With" but does not become the default app.
- Windows gets no file associations, because the app does not read
  command-line arguments.
- If Tauri's `fileAssociations` cannot declare the folder type, add it with a
  custom `Info.plist` fragment. Verify this during implementation.
- A Dock drop can start the app before the UI is ready. The backend stores
  the path in a "pending open" slot. If the UI is running, the backend also
  emits an event. At startup, the UI calls `take_pending_open`.

## 5. Data flow

1. A path arrives from a dialog, a window drop, or a Dock drop.
2. `source` detects the type and creates a `PageSource`.
3. `book` lists entries and keeps only image files that are not excluded.
4. `book` sorts entries with `natural_cmp`.
5. `book` reads each page's width × height from the header. If this fails,
   the size is unknown: the UI uses a 2:3 placeholder and corrects it when the
   image loads.
6. The UI receives `BookInfo` and builds the placeholders.
7. On scroll, the UI sets `comic://` URLs. `protocol` reads one page and
   returns its bytes.

## 6. UI behavior

### 6.1 Layout

- Toolbar at the top. One centered column of pages below it.
- No book open: show "Drop a comic or folder here" and the two Open buttons.
- Drag over the window: show a drop highlight.
- Window title: the book name.

### 6.2 Zoom

- Column width = zoom % × window width. Range 25–200%, step 10%. Default 100%.
- Above 100%, a horizontal scroll bar shows.
- Controls: toolbar buttons, `Cmd/Ctrl +`, `Cmd/Ctrl −`, `Cmd/Ctrl 0`
  (reset to 100%).
- When the zoom changes, the current page stays at the same screen position.
- The zoom level is saved in `localStorage` and restored at the next start.
  Wrap access in try/catch; fall back to 100%.

### 6.3 Full screen

- Toolbar button, `F11` on Windows, `Ctrl+Cmd+F` on macOS. `Esc` exits.
- In full screen, the toolbar hides and the page counter shows as a small
  overlay. Moving the mouse to the top edge shows the toolbar.

### 6.4 Page counter

- Format: `12 / 48`.
- The current page is the page at the vertical center of the viewport.
- Updates during scroll, at most once per animation frame.

## 7. Error handling

| Case | Behavior |
|---|---|
| Opening a book | Runs on a background thread. RAR / tar.gz shows "Extracting…". |
| Second open during an open | The later request wins. The earlier result is discarded and its temporary folder is deleted. |
| Corrupt, unknown, password-protected source, or disk full | Clear error message. The current book stays open. |
| No images in source | "No images found in <name>". The current book stays open. |
| One page fails to load | Its placeholder shows "Page N could not be loaded (<file name>)". Other pages are not affected. |
| Temporary folders | All under one app-owned directory. Deleted when a book closes and when the app exits. Stale folders from a crash are deleted at startup. |

## 8. Security

- Extraction rejects any entry whose resolved path is outside the temporary
  folder (zip slip), including `..` segments and absolute paths.
- The `comic://` handler accepts only a numeric page index for the current
  `book_id`. It never accepts a file path, so the web view cannot read other
  files on disk. Requests for an old `book_id` or an out-of-range index
  return 404.

## 9. Testing

### 9.1 Rust unit tests

- `natural_cmp`: one test per rule in section 2.3, plus the example table.
- Type detection: magic bytes win over the extension.
- Filtering: extension case, hidden files, `__MACOSX/`.
- Zip slip: entries with `..` or absolute paths are rejected.
- Header size: correct width × height for each format; unknown for a broken
  header.

### 9.2 Rust integration tests

- Zip, tar.gz, and folder fixtures are generated at test time.
- `.rar` and `.cbr` fixtures (3–4 small images each) are created with the
  `rar` CLI and committed under `src-tauri/tests/fixtures/`.
- Each test opens the source, checks the sorted page list, and reads every
  page through the same function the protocol handler uses.

### 9.3 Frontend tests (Vitest)

- Zoom limits and steps.
- Scroll anchor calculation on zoom change.
- Current page calculation.

### 9.4 Manual checklist (macOS and Windows)

- Open by each dialog and by window drop. On macOS, also by Dock drop with
  the app closed and with the app running.
- A book with 500+ pages: scroll fast and confirm memory stays stable.
- Full screen, zoom, page counter.
- Each error message in section 7.

## 10. Build and distribution

- macOS output: `.dmg`. Windows output: `.msi` or NSIS installer.
- Each platform builds on its own OS. A Windows machine or VM is required for
  the Windows build and manual tests.
- All builds are local. No CI in version 1.

## 11. Out of scope for version 1

- Code signing and notarization. macOS shows an "unidentified developer"
  warning on first launch.
- Remembered scroll position per book.
- Recent files list.
- Next / previous comic in the same folder.
- Keyboard page navigation (Page Up/Down, Home/End, go to page N).
- Default-app registration.
- Windows command-line open and drop onto `.exe`.
- OCR of page numbers.
- TIFF, HEIC, JPEG XL.
- Plain single-file `.gz`, 7z, and other archive formats.
