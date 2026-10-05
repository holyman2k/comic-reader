# Comic Reader Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Tauri 2 desktop app for macOS and Windows that opens a folder, zip, rar, or tar.gz of images, sorts the pages in natural order, and shows them in one vertical column scaled to the window width.

**Architecture:** A Rust backend opens a source behind a `PageSource` trait (folder, zip, or an extracted temporary folder for rar and tar.gz), sorts and sizes the pages, and serves each page on demand through a custom `comic://` URI scheme. A plain TypeScript frontend builds one placeholder per page with the right aspect ratio and lets an `IntersectionObserver` load and unload images near the viewport.

**Tech Stack:** Tauri 2.12, Rust stable (1.89+), crates `zip` 8.6, `unrar` 0.5.8, `tar` 0.4.46, `flate2` 1.1, `imagesize` 0.15, `tempfile` 3.27, `tauri-plugin-dialog` 2.8; TypeScript, Vite, Vitest.

**Spec:** `docs/superpowers/specs/2026-10-06-comic-reader-design.md`

## Global Constraints

- Sources: folder (with subfolders), `.zip`, `.cbz`, `.rar`, `.cbr`, `.tar.gz`, `.tgz`. Source type is detected by magic bytes first, by extension second.
- Plain single-file `.gz` is not supported. The app is read-only. Password-protected archives produce an error message.
- Image extensions (case-insensitive): `jpg jpeg png gif webp avif bmp`. The backend never decodes or converts image data.
- Excluded entries: any path segment that starts with `.`, and anything under `__MACOSX/`.
- Natural sort rules 1–7 from spec section 2.3, exactly.
- Page URL: `comic://localhost/{book_id}/{index}` on macOS. On Windows Tauri serves the same scheme as `http://comic.localhost/...`, so the frontend must build URLs with `convertFileSrc(\`${bookId}/${index}\`, "comic")`. That function percent-encodes `/` as `%2F`; the backend must accept both forms.
- `book_id` increases on every open. Old `book_id` or out-of-range index returns 404.
- Zoom: 25–200%, step 10%, default 100%, saved in `localStorage` (wrapped in try/catch).
- Full screen: `F11` (Windows), `Ctrl+Cmd+F` (macOS), `Esc` exits. Toolbar hides in full screen and shows when the mouse is at the top edge.
- Page counter format `12 / 48`; the current page is the page at the vertical center of the viewport.
- File associations only in `src-tauri/tauri.macos.conf.json`, role `Viewer`, rank `Alternate`. Windows gets no file associations and no command-line open.
- All temporary folders live under `<system temp>/comic-reader/`. Stale folders from a crash are deleted at startup.
- `generate_context!` embeds `../dist`, so run `npm run build` once before the first `cargo test` (and again after frontend changes if a test build complains that `dist` is missing).
- Rust commands below run from the repo root with `--manifest-path src-tauri/Cargo.toml`.
- Commit messages end with: `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`

## Review Focus

1. **Tarballs made by macOS `tar`** have `./`-prefixed names and AppleDouble `._page.png` files. Expect: the real pages open, the `._` files are excluded. Test in Task 7.
2. **Zip entries written with backslashes** (`ch1\p2.png`, older Windows tools). Expect: treated as folders, sorted per folder, readable. Test in Task 5.
3. **Unicode file names** (Japanese, accented letters). Expect: listed, readable, and sorted case-insensitively with a deterministic tie-break. Tests in Task 2 and Task 4.
4. **Switching books while old pages are still loading.** Expect: requests with the old `book_id` get 404 and never return a page from the new book. Test in Task 10.
5. **Zero-byte or truncated image file.** Expect: the book still opens, that page has unknown size, and reading it returns empty bytes so the UI shows the error placeholder. Test in Task 9.

---

## File Structure

```
.gitignore
package.json                      npm scripts and frontend dependencies
tsconfig.json
vite.config.ts                    Vite dev server (port 1420) and Vitest config
index.html                        static markup: toolbar, scroller, overlays
app-icon.svg                      source for `tauri icon`
scripts/make-rar-fixtures.sh      creates RAR test fixtures with the rar CLI
src/
  main.ts                         wiring: open flow, status, keys, zoom
  types.ts                        BookInfo / PageInfo types, SUPERSEDED constant
  zoom.ts        (+ .test.ts)     zoom limits, steps, persistence
  layout.ts      (+ .test.ts)     current page, scroll anchor math
  keys.ts        (+ .test.ts)     keyboard shortcut mapping
  text.ts        (+ .test.ts)     display name, status label, counter text
  viewer.ts                       page placeholders, lazy load/unload, zoom anchor
  toolbar.ts                      toolbar buttons, zoom label, counter
  fullscreen.ts                   full screen toggle and toolbar reveal
  dragdrop.ts                     window drag-and-drop
  styles.css
src-tauri/
  Cargo.toml
  build.rs
  tauri.conf.json
  tauri.macos.conf.json           macOS file associations
  capabilities/default.json
  icons/                          generated by `npx tauri icon`
  src/
    main.rs                       binary entry, calls lib::run
    lib.rs                        Tauri builder, protocol registration, run events
    natural_sort.rs               natural_cmp
    image_entry.rs                entry name rules, content type, header size
    source/mod.rs                 PageSource trait, SourceError, detection, safe paths
    source/folder.rs              FolderSource
    source/zip_source.rs          ZipSource
    source/extract.rs             ExtractedSource (tar.gz, rar)
    temp.rs                       per-instance temp folder with lock, stale cleanup
    book.rs                       Book::open, PageInfo, BookInfo, OpenError
    state.rs                      AppState: current book, tickets, shutdown
    protocol.rs                   comic:// path parsing and responses
    commands.rs                   open_book, take_pending_open
    launch_open.rs                PendingOpen store and event name
  tests/
    common/mod.rs                 test helpers: png bytes, zip / tar.gz builders
    sources.rs                    folder, zip, tar.gz, rar integration tests
    book.rs                       Book::open integration tests
    app.rs                        AppState and protocol integration tests
    fixtures/                     book.rar, book.cbr, encrypted-*.rar
```

---

### Task 1: Project scaffold

**Files:**
- Create: `.gitignore`, `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `app-icon.svg`, `src/main.ts`, `src/styles.css`, `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Generate: `src-tauri/icons/*`

**Interfaces:**
- Produces: crate `comic_reader_lib` with `pub fn run()`; npm scripts `dev`, `build`, `test`, `tauri`; HTML element ids used by later tasks: `toolbar`, `open-file`, `open-folder`, `zoom-out`, `zoom-label`, `zoom-in`, `fullscreen`, `counter`, `scroller`, `empty`, `pages`, `counter-overlay`, `status`, `drop-highlight`; `[data-open=file]`, `[data-open=folder]` buttons in the empty state.

- [ ] **Step 1: Create `.gitignore`**

```gitignore
node_modules/
dist/
src-tauri/target/
src-tauri/gen/
.DS_Store
```

- [ ] **Step 2: Create `package.json`**

```json
{
  "name": "comic-reader",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "test": "vitest run",
    "tauri": "tauri"
  }
}
```

- [ ] **Step 3: Install frontend dependencies**

Run:
```bash
npm install @tauri-apps/api@^2.12 @tauri-apps/plugin-dialog@^2.8
npm install -D @tauri-apps/cli@^2.12 vite vitest typescript
```
Expected: `package.json` gains `dependencies` and `devDependencies`; `package-lock.json` is created.

- [ ] **Step 4: Create `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "moduleResolution": "bundler",
    "skipLibCheck": true,
    "isolatedModules": true,
    "noEmit": true,
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true
  },
  "include": ["src"]
}
```

- [ ] **Step 5: Create `vite.config.ts`**

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
  },
});
```

- [ ] **Step 6: Create `index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Comic Reader</title>
    <link rel="stylesheet" href="/src/styles.css" />
    <script type="module" src="/src/main.ts"></script>
  </head>
  <body>
    <header id="toolbar">
      <button id="open-file">Open File</button>
      <button id="open-folder">Open Folder</button>
      <span class="spacer"></span>
      <button id="zoom-out" title="Zoom out">−</button>
      <span id="zoom-label">100%</span>
      <button id="zoom-in" title="Zoom in">+</button>
      <button id="fullscreen" title="Full screen">Full Screen</button>
      <span id="counter"></span>
    </header>
    <main id="scroller">
      <div id="empty">
        <p>Drop a comic or folder here</p>
        <div>
          <button data-open="file">Open File</button>
          <button data-open="folder">Open Folder</button>
        </div>
      </div>
      <div id="pages"></div>
    </main>
    <div id="counter-overlay"></div>
    <div id="status" hidden></div>
    <div id="drop-highlight" hidden>Drop to open</div>
  </body>
</html>
```

- [ ] **Step 7: Create `src/styles.css`**

```css
:root {
  --bg: #111214;
  --panel: #1c1d21;
  --border: #3a3d44;
  --text: #e6e6e6;
  --muted: #9a9ca3;
  --accent: #4c8dff;
  --error: #ff6b6b;
  --zoom: 100;
  color-scheme: dark;
}

* { box-sizing: border-box; }

html, body {
  margin: 0;
  height: 100%;
  background: var(--bg);
  color: var(--text);
  font: 13px/1.4 system-ui, -apple-system, "Segoe UI", sans-serif;
}

body { display: flex; flex-direction: column; overflow: hidden; }

#toolbar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  background: var(--panel);
  border-bottom: 1px solid #000;
  flex: none;
  z-index: 10;
}

button {
  background: #2a2c31;
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 4px 10px;
  font: inherit;
  cursor: pointer;
}
button:hover { background: #34373d; }

#toolbar .spacer { flex: 1; }
#zoom-label { min-width: 44px; text-align: center; color: var(--muted); }
#counter { min-width: 70px; text-align: right; color: var(--muted); }

#scroller { position: relative; flex: 1; overflow: auto; }
#pages { width: calc(var(--zoom) * 1%); margin: 0 auto; }

.page { position: relative; width: 100%; background: #1a1b1e; }
.page img { display: block; width: 100%; height: 100%; }
.page img:not([src]), .page.failed img { visibility: hidden; }
.page.failed::after {
  content: attr(data-message);
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
  color: var(--error);
  text-align: center;
}

#empty {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--muted);
  font-size: 15px;
}
#empty div { display: flex; gap: 8px; }
body.has-book #empty { display: none; }

#drop-highlight {
  position: fixed;
  inset: 8px;
  border: 3px dashed var(--accent);
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  color: var(--accent);
  background: rgb(76 141 255 / 0.08);
  pointer-events: none;
  z-index: 30;
}

#status {
  position: fixed;
  left: 50%;
  bottom: 24px;
  transform: translateX(-50%);
  max-width: min(90vw, 560px);
  padding: 10px 16px;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 6px;
  z-index: 20;
}
#status.error { border-color: var(--error); color: var(--error); cursor: pointer; }

#counter-overlay {
  display: none;
  position: fixed;
  right: 12px;
  bottom: 12px;
  padding: 4px 10px;
  background: rgb(0 0 0 / 0.6);
  border-radius: 4px;
  z-index: 15;
  pointer-events: none;
}

body.fullscreen #toolbar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  transform: translateY(-100%);
  transition: transform 0.15s;
}
body.fullscreen.reveal-toolbar #toolbar { transform: none; }
body.fullscreen.has-book #counter-overlay { display: block; }

[hidden] { display: none !important; }
```

- [ ] **Step 8: Create placeholder `src/main.ts`**

```ts
// Wiring is added in Task 12.
export {};
```

- [ ] **Step 9: Create `app-icon.svg`**

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <rect x="64" y="64" width="896" height="896" rx="180" fill="#1c1d21"/>
  <rect x="300" y="170" width="424" height="200" rx="16" fill="#4c8dff"/>
  <rect x="300" y="412" width="424" height="200" rx="16" fill="#e6e6e6"/>
  <rect x="300" y="654" width="424" height="200" rx="16" fill="#4c8dff"/>
</svg>
```

- [ ] **Step 10: Create `src-tauri/Cargo.toml`**

```toml
[package]
name = "comic-reader"
version = "0.1.0"
edition = "2021"
rust-version = "1.89"

[lib]
name = "comic_reader_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2.7", features = [] }

[dependencies]
tauri = { version = "2.12", features = [] }
tauri-plugin-dialog = "2.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
zip = "8.6"
unrar = "0.5.8"
tar = "0.4.46"
flate2 = "1.1"
imagesize = "0.15"
tempfile = "3.27"
```

`rust-version = "1.89"` is required because `temp.rs` uses `std::fs::File::try_lock`.

- [ ] **Step 11: Create `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 12: Create `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Comic Reader",
  "version": "0.1.0",
  "identifier": "com.comicreader.viewer",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "Comic Reader",
        "width": 1000,
        "height": 900,
        "minWidth": 400,
        "minHeight": 300
      }
    ],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

The identifier must not end in `.app` (Tauri warns that it conflicts with the macOS bundle extension).

- [ ] **Step 13: Create `src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Permissions for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-set-fullscreen",
    "core:window:allow-set-title",
    "dialog:allow-open"
  ]
}
```

- [ ] **Step 14: Create `src-tauri/src/main.rs`**

```rust
// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    comic_reader_lib::run()
}
```

- [ ] **Step 15: Create minimal `src-tauri/src/lib.rs`**

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 16: Generate icons**

Run: `npx tauri icon app-icon.svg`
Expected: `src-tauri/icons/` contains `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico`.

- [ ] **Step 17: Build the frontend and check the Rust crate**

Run: `npm run build && cargo check --manifest-path src-tauri/Cargo.toml`
Expected: `dist/` is created; `cargo check` finishes with no errors. The first run compiles `unrar` C++ code and takes several minutes.

- [ ] **Step 18: Run the app once**

Run: `npm run tauri dev`
Expected: a window titled "Comic Reader" opens with the toolbar and the text "Drop a comic or folder here". Close the window.

- [ ] **Step 19: Commit**

```bash
git add .gitignore package.json package-lock.json tsconfig.json vite.config.ts index.html app-icon.svg src src-tauri
git commit -m "chore: scaffold Tauri 2 app with TypeScript frontend

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Natural sort

**Files:**
- Create: `src-tauri/src/natural_sort.rs`
- Modify: `src-tauri/src/lib.rs` (add module)

**Interfaces:**
- Produces: `pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering` in module `comic_reader_lib::natural_sort`. Paths use `/` separators.

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/src/natural_sort.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::natural_cmp;
    use std::cmp::Ordering;

    fn sorted<'a>(input: &[&'a str]) -> Vec<&'a str> {
        let mut v = input.to_vec();
        v.sort_by(|a, b| natural_cmp(a, b));
        v
    }

    #[test]
    fn numbers_sort_by_value() {
        assert_eq!(sorted(&["page 10", "page 2", "page 1"]), ["page 1", "page 2", "page 10"]);
        assert_eq!(sorted(&["p010", "p9", "p1"]), ["p1", "p9", "p010"]);
    }

    #[test]
    fn folders_compare_one_level_at_a_time() {
        assert_eq!(sorted(&["Ch2/01", "ch10/01", "ch2/02"]), ["Ch2/01", "ch2/02", "ch10/01"]);
        assert_eq!(
            sorted(&["vol1 p2", "vol1 p10", "vol2 p1"]),
            ["vol1 p2", "vol1 p10", "vol2 p1"]
        );
    }

    #[test]
    fn text_is_case_insensitive() {
        assert_eq!(sorted(&["b", "A", "c"]), ["A", "b", "c"]);
    }

    #[test]
    fn number_before_text_at_same_position() {
        assert_eq!(sorted(&["a.jpg", "1.jpg"]), ["1.jpg", "a.jpg"]);
    }

    #[test]
    fn fewer_leading_zeros_first_then_bytes() {
        assert_eq!(sorted(&["01.png", "1.png", "001.png"]), ["1.png", "01.png", "001.png"]);
        assert_eq!(sorted(&["a.png", "A.png"]), ["A.png", "a.png"]);
    }

    #[test]
    fn punctuation_is_text() {
        assert_eq!(sorted(&["1.10", "1.5"]), ["1.5", "1.10"]);
        assert_eq!(sorted(&["-3", "-1"]), ["-1", "-3"]);
    }

    #[test]
    fn long_numbers_do_not_overflow() {
        assert_eq!(
            sorted(&["p100000000000000000000000", "p99999999999999999999999"]),
            ["p99999999999999999999999", "p100000000000000000000000"]
        );
    }

    #[test]
    fn unicode_names_sort_case_insensitively_and_deterministically() {
        assert_eq!(
            sorted(&["éclair.png", "Zebra.png", "apple.png"]),
            ["apple.png", "Zebra.png", "éclair.png"]
        );
        assert_eq!(sorted(&["élan.png", "Élan.png"]), ["Élan.png", "élan.png"]);
        assert_eq!(sorted(&["ページ10.png", "ページ2.png"]), ["ページ2.png", "ページ10.png"]);
    }

    #[test]
    fn order_is_total_and_antisymmetric() {
        let names = [
            "a", "A", "a1", "a01", "a/1", "a/01", "A/1", "1", "01", "", "ä", "a.png", "a 1",
        ];
        for x in names {
            assert_eq!(natural_cmp(x, x), Ordering::Equal, "{x}");
            for y in names {
                assert_eq!(natural_cmp(x, y), natural_cmp(y, x).reverse(), "{x} vs {y}");
                if x != y {
                    assert_ne!(natural_cmp(x, y), Ordering::Equal, "{x} vs {y}");
                }
            }
        }
    }
}
```

Add to the top of `src-tauri/src/lib.rs`:

```rust
pub mod natural_sort;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib natural_sort`
Expected: compile error `cannot find function natural_cmp`.

- [ ] **Step 3: Write the implementation**

Insert above the test module in `src-tauri/src/natural_sort.rs`:

```rust
//! Natural sort order for page paths: text runs compare case-insensitively,
//! digit runs compare by numeric value, one folder level at a time.

use std::cmp::Ordering;

#[derive(Clone, Copy)]
enum Token<'a> {
    Text(&'a str),
    Number(&'a str),
}

/// Compares two `/`-separated paths in natural order. The order is total:
/// it returns `Equal` only for identical strings.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let ta = path_tokens(a);
    let tb = path_tokens(b);
    primary(&ta, &tb)
        .then_with(|| leading_zeros(&ta, &tb))
        .then_with(|| a.cmp(b))
}

fn path_tokens(path: &str) -> Vec<Vec<Token<'_>>> {
    path.split('/').map(segment_tokens).collect()
}

fn segment_tokens(segment: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut run_is_digits: Option<bool> = None;
    for (i, ch) in segment.char_indices() {
        let digit = ch.is_ascii_digit();
        if let Some(prev) = run_is_digits {
            if prev != digit {
                out.push(make_token(&segment[start..i], prev));
                start = i;
            }
        }
        run_is_digits = Some(digit);
    }
    if let Some(prev) = run_is_digits {
        out.push(make_token(&segment[start..], prev));
    }
    out
}

fn make_token(s: &str, digits: bool) -> Token<'_> {
    if digits {
        Token::Number(s)
    } else {
        Token::Text(s)
    }
}

fn cmp_text(a: &str, b: &str) -> Ordering {
    a.chars()
        .flat_map(char::to_lowercase)
        .cmp(b.chars().flat_map(char::to_lowercase))
}

fn cmp_number(a: &str, b: &str) -> Ordering {
    let a = a.trim_start_matches('0');
    let b = b.trim_start_matches('0');
    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
}

fn cmp_token(a: Token, b: Token) -> Ordering {
    match (a, b) {
        (Token::Number(x), Token::Number(y)) => cmp_number(x, y),
        (Token::Text(x), Token::Text(y)) => cmp_text(x, y),
        (Token::Number(_), Token::Text(_)) => Ordering::Less,
        (Token::Text(_), Token::Number(_)) => Ordering::Greater,
    }
}

fn primary(a: &[Vec<Token>], b: &[Vec<Token>]) -> Ordering {
    for (sa, sb) in a.iter().zip(b) {
        let order = sa
            .iter()
            .zip(sb)
            .map(|(x, y)| cmp_token(*x, *y))
            .find(|o| o.is_ne())
            .unwrap_or_else(|| sa.len().cmp(&sb.len()));
        if order.is_ne() {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

fn leading_zeros(a: &[Vec<Token>], b: &[Vec<Token>]) -> Ordering {
    for (sa, sb) in a.iter().zip(b) {
        for (x, y) in sa.iter().zip(sb) {
            if let (Token::Number(x), Token::Number(y)) = (x, y) {
                let order = x.len().cmp(&y.len());
                if order.is_ne() {
                    return order;
                }
            }
        }
    }
    Ordering::Equal
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib natural_sort`
Expected: 9 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/natural_sort.rs src-tauri/src/lib.rs
git commit -m "feat: add natural sort for page paths

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Image entry rules and header size

**Files:**
- Create: `src-tauri/src/image_entry.rs`
- Modify: `src-tauri/src/lib.rs` (add module)

**Interfaces:**
- Produces (module `comic_reader_lib::image_entry`):
  - `pub fn normalize_entry_name(name: &str) -> String` — `\` becomes `/`; empty and `.` segments are removed.
  - `pub fn is_page_entry(path: &str) -> bool` — image extension and not excluded. Expects a normalized path.
  - `pub fn content_type(path: &str) -> &'static str`
  - `pub fn page_size(bytes: &[u8]) -> Option<(u32, u32)>` — width and height from the image header; `None` if unreadable or zero.

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/src/image_entry.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_separators_and_dot_segments() {
        assert_eq!(normalize_entry_name("ch1\\p2.png"), "ch1/p2.png");
        assert_eq!(normalize_entry_name("./ch1//p2.png"), "ch1/p2.png");
        assert_eq!(normalize_entry_name("../x.png"), "../x.png");
    }

    #[test]
    fn accepts_image_extensions_in_any_case() {
        for name in ["a.jpg", "a.JPEG", "b/c.Png", "d.gif", "e.webp", "f.AVIF", "g.bmp"] {
            assert!(is_page_entry(name), "{name}");
        }
    }

    #[test]
    fn rejects_other_files_and_folder_dots() {
        for name in ["notes.txt", "png", "a.png/readme", "dir.jpg/file", "a.tiff", ""] {
            assert!(!is_page_entry(name), "{name}");
        }
    }

    #[test]
    fn excludes_hidden_and_macosx_entries() {
        for name in [".hidden.png", "._p1.png", ".git/x.png", "__MACOSX/p1.png", "a/__MACOSX/b.png"] {
            assert!(!is_page_entry(name), "{name}");
        }
    }

    #[test]
    fn content_types() {
        assert_eq!(content_type("a/B.JPG"), "image/jpeg");
        assert_eq!(content_type("a.jpeg"), "image/jpeg");
        assert_eq!(content_type("a.png"), "image/png");
        assert_eq!(content_type("a.gif"), "image/gif");
        assert_eq!(content_type("a.webp"), "image/webp");
        assert_eq!(content_type("a.avif"), "image/avif");
        assert_eq!(content_type("a.bmp"), "image/bmp");
        assert_eq!(content_type("a.txt"), "application/octet-stream");
    }

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&w.to_be_bytes());
        v.extend_from_slice(&h.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        v
    }

    #[test]
    fn reads_png_size() {
        assert_eq!(page_size(&png(800, 1200)), Some((800, 1200)));
    }

    #[test]
    fn reads_gif_size() {
        let gif = [b'G', b'I', b'F', b'8', b'9', b'a', 0x20, 0x03, 0x58, 0x02, 0, 0, 0];
        assert_eq!(page_size(&gif), Some((800, 600)));
    }

    #[test]
    fn reads_bmp_size() {
        let mut bmp = vec![b'B', b'M'];
        bmp.extend_from_slice(&[0; 12]); // file size, reserved, data offset
        bmp.extend_from_slice(&40u32.to_le_bytes()); // DIB header size
        bmp.extend_from_slice(&640i32.to_le_bytes());
        bmp.extend_from_slice(&480i32.to_le_bytes());
        bmp.extend_from_slice(&[1, 0, 24, 0]);
        bmp.extend_from_slice(&[0; 24]);
        assert_eq!(page_size(&bmp), Some((640, 480)));
    }

    #[test]
    fn reads_jpeg_size() {
        // SOI, then a baseline SOF0 segment: length 17, precision 8, height 300, width 200.
        let jpeg = [
            0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x01, 0x2C, 0x00, 0xC8, 0x03, 0x01, 0x22,
            0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xFF, 0xD9,
        ];
        assert_eq!(page_size(&jpeg), Some((200, 300)));
    }

    #[test]
    fn reads_webp_size() {
        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&30u32.to_le_bytes());
        webp.extend_from_slice(b"WEBPVP8X");
        webp.extend_from_slice(&10u32.to_le_bytes());
        webp.extend_from_slice(&[0, 0, 0, 0]);
        webp.extend_from_slice(&[0x1F, 0x03, 0x00]); // width - 1 = 799
        webp.extend_from_slice(&[0xAF, 0x04, 0x00]); // height - 1 = 1199
        assert_eq!(page_size(&webp), Some((800, 1200)));
    }

    #[test]
    fn unreadable_header_has_no_size() {
        assert_eq!(page_size(&[]), None);
        assert_eq!(page_size(b"not an image"), None);
        assert_eq!(page_size(&png(800, 1200)[..10]), None);
        assert_eq!(page_size(&png(0, 1200)), None);
    }
}
```

AVIF header parsing is covered by the `imagesize` crate's own tests; building a valid AVIF box tree by hand adds no value here.

Add to `src-tauri/src/lib.rs`:

```rust
pub mod image_entry;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib image_entry`
Expected: compile errors for the missing functions.

- [ ] **Step 3: Write the implementation**

Insert above the test module:

```rust
//! Rules for which archive or folder entries are pages, and header probing.

const IMAGE_EXTENSIONS: [&str; 7] = ["jpg", "jpeg", "png", "gif", "webp", "avif", "bmp"];

/// Converts `\` to `/` and drops empty and `.` segments. `..` is kept so that
/// callers can still reject it.
pub fn normalize_entry_name(name: &str) -> String {
    name.split(['/', '\\'])
        .filter(|seg| !seg.is_empty() && *seg != ".")
        .collect::<Vec<_>>()
        .join("/")
}

fn extension(path: &str) -> Option<String> {
    let file = path.rsplit('/').next()?;
    let (stem, ext) = file.rsplit_once('.')?;
    (!stem.is_empty() || !ext.is_empty()).then(|| ext.to_ascii_lowercase())
}

fn is_excluded(path: &str) -> bool {
    path.split('/')
        .any(|seg| seg == "__MACOSX" || (seg.starts_with('.') && seg != "." && seg != ".."))
}

/// True when `path` (normalized, `/`-separated) is an image that is not excluded.
pub fn is_page_entry(path: &str) -> bool {
    let is_image = extension(path).is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.as_str()));
    is_image && !is_excluded(path)
}

pub fn content_type(path: &str) -> &'static str {
    match extension(path).as_deref() {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("bmp") => "image/bmp",
        _ => "application/octet-stream",
    }
}

/// Width and height from the image header, or `None` if it cannot be read.
pub fn page_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let size = imagesize::blob_size(bytes).ok()?;
    if size.width == 0 || size.height == 0 {
        return None;
    }
    Some((u32::try_from(size.width).ok()?, u32::try_from(size.height).ok()?))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib image_entry`
Expected: 11 tests PASS. If one header test fails, check the test bytes against the `imagesize` parser for that format before you change `page_size`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/image_entry.rs src-tauri/src/lib.rs
git commit -m "feat: add page entry rules and header size probing

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: PageSource trait, detection, and FolderSource

**Files:**
- Create: `src-tauri/src/source/mod.rs`, `src-tauri/src/source/folder.rs`, `src-tauri/tests/common/mod.rs`, `src-tauri/tests/sources.rs`
- Modify: `src-tauri/src/lib.rs` (add module)

**Interfaces:**
- Produces (module `comic_reader_lib::source`):
  - `pub enum SourceError { Io(std::io::Error), Unsupported, Corrupt(String), Encrypted, NotFound(String) }` with `Display` and `From<io::Error>`.
  - `pub trait PageSource: Send + Sync { fn list(&self) -> Result<Vec<String>, SourceError>; fn read(&self, path: &str) -> Result<Vec<u8>, SourceError>; fn read_prefix(&self, path: &str, limit: usize) -> Result<Vec<u8>, SourceError>; }` — paths are relative, `/`-separated, as returned by `list`.
  - `pub enum SourceKind { Folder, Zip, Rar, TarGz }`, `pub fn kind_from_magic(head: &[u8]) -> Option<SourceKind>`, `pub fn kind_from_extension(name: &str) -> Option<SourceKind>`, `pub fn detect(path: &Path) -> Result<SourceKind, SourceError>`.
  - `pub fn safe_relative_path(name: &str) -> Option<PathBuf>` — `None` for absolute paths, drive prefixes, and `..`.
  - `pub struct FolderSource` with `pub fn new(root: impl Into<PathBuf>) -> Self`.
  - `pub fn open_source(path: &Path, temp_base: &Path) -> Result<Box<dyn PageSource>, SourceError>` — in this task it supports only `Folder`; Tasks 5, 7, and 8 add the other kinds.
- Test helpers (`tests/common/mod.rs`): `png(w, h) -> Vec<u8>`, `write_folder(root, files)`, `zip_bytes(files)`, `tar_gz_bytes(files)`, `fixture(name) -> PathBuf`, `temp_base() -> tempfile::TempDir`.

- [ ] **Step 1: Create the test helpers**

Create `src-tauri/tests/common/mod.rs`:

```rust
#![allow(dead_code)]

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

/// Signature + IHDR chunk. Enough for header probing; not a decodable image.
pub fn png(width: u32, height: u32) -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
    v.extend_from_slice(b"IHDR");
    v.extend_from_slice(&width.to_be_bytes());
    v.extend_from_slice(&height.to_be_bytes());
    v.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    v
}

pub fn write_folder(root: &Path, files: &[(&str, Vec<u8>)]) {
    for (name, data) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, data).unwrap();
    }
}

pub fn zip_bytes(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in files {
        w.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap().into_inner()
}

/// Writes raw names into the tar header, so tests can use `./` and `../` names.
pub fn tar_gz_bytes(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut builder = tar::Builder::new(enc);
    for (name, data) in files {
        let mut header = tar::Header::new_gnu();
        let raw = name.as_bytes();
        header.as_old_mut().name[..raw.len()].copy_from_slice(raw);
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        builder.append(&header, &data[..]).unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

pub fn temp_base() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}
```

- [ ] **Step 2: Write the failing unit tests for detection and safe paths**

Create `src-tauri/src/source/mod.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_bytes() {
        assert_eq!(kind_from_magic(b"PK\x03\x04rest"), Some(SourceKind::Zip));
        assert_eq!(kind_from_magic(b"PK\x05\x06"), Some(SourceKind::Zip));
        assert_eq!(kind_from_magic(b"Rar!\x1a\x07\x00"), Some(SourceKind::Rar));
        assert_eq!(kind_from_magic(b"Rar!\x1a\x07\x01\x00"), Some(SourceKind::Rar));
        assert_eq!(kind_from_magic(&[0x1f, 0x8b, 8, 0]), Some(SourceKind::TarGz));
        assert_eq!(kind_from_magic(b"hello"), None);
        assert_eq!(kind_from_magic(b""), None);
    }

    #[test]
    fn extensions() {
        assert_eq!(kind_from_extension("A.CBZ"), Some(SourceKind::Zip));
        assert_eq!(kind_from_extension("a.zip"), Some(SourceKind::Zip));
        assert_eq!(kind_from_extension("a.cbr"), Some(SourceKind::Rar));
        assert_eq!(kind_from_extension("a.RAR"), Some(SourceKind::Rar));
        assert_eq!(kind_from_extension("a.tar.gz"), Some(SourceKind::TarGz));
        assert_eq!(kind_from_extension("a.tgz"), Some(SourceKind::TarGz));
        assert_eq!(kind_from_extension("a.7z"), None);
    }

    #[test]
    fn safe_paths() {
        assert_eq!(safe_relative_path("a/b.png"), Some(PathBuf::from("a").join("b.png")));
        assert_eq!(safe_relative_path("./a\\b.png"), Some(PathBuf::from("a").join("b.png")));
        assert_eq!(safe_relative_path("../b.png"), None);
        assert_eq!(safe_relative_path("a/../../b.png"), None);
        assert_eq!(safe_relative_path("/etc/passwd"), None);
        assert_eq!(safe_relative_path("\\windows\\x.png"), None);
        assert_eq!(safe_relative_path("C:/x.png"), None);
        assert_eq!(safe_relative_path("./"), None);
    }
}
```

Add to `src-tauri/src/lib.rs`:

```rust
pub mod source;
```

- [ ] **Step 3: Write the failing integration tests for FolderSource**

Create `src-tauri/tests/sources.rs`:

```rust
mod common;

use comic_reader_lib::source::{detect, open_source, PageSource, SourceError, SourceKind};
use common::*;

fn sorted_list(source: &dyn PageSource) -> Vec<String> {
    let mut names = source.list().unwrap();
    names.sort();
    names
}

#[test]
fn folder_lists_files_recursively_with_slashes() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("p1.png", png(1, 2)), ("ch1/p2.png", png(3, 4)), ("notes.txt", b"x".to_vec())]);
    let base = temp_base();
    let source = open_source(dir.path(), base.path()).unwrap();
    assert_eq!(sorted_list(source.as_ref()), ["ch1/p2.png", "notes.txt", "p1.png"]);
    assert_eq!(source.read("ch1/p2.png").unwrap(), png(3, 4));
    assert_eq!(source.read_prefix("ch1/p2.png", 8).unwrap(), png(3, 4)[..8].to_vec());
}

#[test]
fn folder_reads_unicode_names() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("第1話/ページ2.png", png(5, 6)), ("Élan.png", png(7, 8))]);
    let base = temp_base();
    let source = open_source(dir.path(), base.path()).unwrap();
    let names = sorted_list(source.as_ref());
    assert_eq!(names.len(), 2);
    for name in &names {
        assert!(!source.read(name).unwrap().is_empty(), "{name}");
    }
}

#[test]
fn folder_rejects_unsafe_and_missing_paths() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("p1.png", png(1, 2))]);
    let base = temp_base();
    let source = open_source(dir.path(), base.path()).unwrap();
    assert!(matches!(source.read("../p1.png"), Err(SourceError::NotFound(_))));
    assert!(matches!(source.read("missing.png"), Err(SourceError::NotFound(_))));
}

#[test]
fn detect_prefers_magic_bytes_over_extension() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("really-a-zip.cbr");
    std::fs::write(&path, zip_bytes(&[("p1.png", png(1, 2))])).unwrap();
    assert_eq!(detect(&path).unwrap(), SourceKind::Zip);
    assert_eq!(detect(dir.path()).unwrap(), SourceKind::Folder);
}

#[test]
fn detect_unknown_file_is_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.txt");
    std::fs::write(&path, b"hello").unwrap();
    assert!(matches!(detect(&path), Err(SourceError::Unsupported)));
}

#[test]
fn detect_missing_file_is_io_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(detect(&dir.path().join("gone.cbz")), Err(SourceError::Io(_))));
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib source && cargo test --manifest-path src-tauri/Cargo.toml --test sources`
Expected: compile errors for the missing items.

- [ ] **Step 5: Implement `source/mod.rs`**

Insert above the test module in `src-tauri/src/source/mod.rs`:

```rust
//! Page sources: a folder, a zip, or an archive extracted to a temporary folder.

mod folder;

pub use folder::FolderSource;

use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum SourceError {
    Io(io::Error),
    Unsupported,
    Corrupt(String),
    Encrypted,
    NotFound(String),
}

impl From<io::Error> for SourceError {
    fn from(err: io::Error) -> Self {
        SourceError::Io(err)
    }
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::Io(err) => write!(f, "I/O error: {err}"),
            SourceError::Unsupported => write!(f, "unsupported source"),
            SourceError::Corrupt(detail) => write!(f, "damaged archive: {detail}"),
            SourceError::Encrypted => write!(f, "password protected"),
            SourceError::NotFound(path) => write!(f, "{path} not found"),
        }
    }
}

/// A set of files that can be listed and read. Paths are relative and `/`-separated.
pub trait PageSource: Send + Sync {
    fn list(&self) -> Result<Vec<String>, SourceError>;
    fn read(&self, path: &str) -> Result<Vec<u8>, SourceError>;
    /// Reads at most `limit` bytes from the start of the file.
    fn read_prefix(&self, path: &str, limit: usize) -> Result<Vec<u8>, SourceError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Folder,
    Zip,
    Rar,
    TarGz,
}

pub fn kind_from_magic(head: &[u8]) -> Option<SourceKind> {
    if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        Some(SourceKind::Zip)
    } else if head.starts_with(b"Rar!\x1a\x07") {
        Some(SourceKind::Rar)
    } else if head.starts_with(&[0x1f, 0x8b]) {
        Some(SourceKind::TarGz)
    } else {
        None
    }
}

pub fn kind_from_extension(name: &str) -> Option<SourceKind> {
    let name = name.to_ascii_lowercase();
    if name.ends_with(".zip") || name.ends_with(".cbz") {
        Some(SourceKind::Zip)
    } else if name.ends_with(".rar") || name.ends_with(".cbr") {
        Some(SourceKind::Rar)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        Some(SourceKind::TarGz)
    } else {
        None
    }
}

/// Detects the source type: folder, then magic bytes, then extension.
pub fn detect(path: &Path) -> Result<SourceKind, SourceError> {
    if path.is_dir() {
        return Ok(SourceKind::Folder);
    }
    let mut head = Vec::with_capacity(8);
    File::open(path)?.take(8).read_to_end(&mut head)?;
    kind_from_magic(&head)
        .or_else(|| kind_from_extension(&path.to_string_lossy()))
        .ok_or(SourceError::Unsupported)
}

/// Converts an entry name to a relative path. Returns `None` for absolute
/// paths, drive prefixes, `..` segments, and empty names.
pub fn safe_relative_path(name: &str) -> Option<PathBuf> {
    if name.starts_with('/') || name.starts_with('\\') {
        return None;
    }
    let mut out = PathBuf::new();
    for seg in name.split(['/', '\\']) {
        match seg {
            "" | "." => continue,
            ".." => return None,
            s if s.contains(':') => return None,
            s => out.push(s),
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

pub fn open_source(path: &Path, _temp_base: &Path) -> Result<Box<dyn PageSource>, SourceError> {
    match detect(path)? {
        SourceKind::Folder => Ok(Box::new(FolderSource::new(path))),
        _ => Err(SourceError::Unsupported),
    }
}
```

- [ ] **Step 6: Implement `source/folder.rs`**

```rust
use super::{safe_relative_path, PageSource, SourceError};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub struct FolderSource {
    root: PathBuf,
}

impl FolderSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn resolve(&self, path: &str) -> Result<PathBuf, SourceError> {
        let rel = safe_relative_path(path).ok_or_else(|| SourceError::NotFound(path.to_owned()))?;
        Ok(self.root.join(rel))
    }
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            walk(&entry.path(), &rel, out)?;
        } else if file_type.is_file() || (file_type.is_symlink() && entry.path().is_file()) {
            out.push(rel);
        }
    }
    Ok(())
}

fn not_found_or_io(path: &str, err: io::Error) -> SourceError {
    if err.kind() == io::ErrorKind::NotFound {
        SourceError::NotFound(path.to_owned())
    } else {
        SourceError::Io(err)
    }
}

impl PageSource for FolderSource {
    fn list(&self) -> Result<Vec<String>, SourceError> {
        let mut out = Vec::new();
        walk(&self.root, "", &mut out)?;
        Ok(out)
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, SourceError> {
        fs::read(self.resolve(path)?).map_err(|e| not_found_or_io(path, e))
    }

    fn read_prefix(&self, path: &str, limit: usize) -> Result<Vec<u8>, SourceError> {
        let file = File::open(self.resolve(path)?).map_err(|e| not_found_or_io(path, e))?;
        let mut buf = Vec::new();
        file.take(limit as u64).read_to_end(&mut buf)?;
        Ok(buf)
    }
}
```

Symlinked folders are not followed, so a link loop cannot hang the listing.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib source && cargo test --manifest-path src-tauri/Cargo.toml --test sources`
Expected: 3 unit tests and 6 integration tests PASS.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/source src-tauri/src/lib.rs src-tauri/tests
git commit -m "feat: add page source trait, type detection, and folder source

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: ZipSource

**Files:**
- Create: `src-tauri/src/source/zip_source.rs`
- Modify: `src-tauri/src/source/mod.rs`, `src-tauri/tests/sources.rs`

**Interfaces:**
- Consumes: `PageSource`, `SourceError` (Task 4); `normalize_entry_name` (Task 3).
- Produces: `pub struct ZipSource` with `pub fn open(path: &Path) -> Result<ZipSource, SourceError>`; `open_source` returns it for `SourceKind::Zip`. Listed names are normalized (`\` → `/`, no `./`).

- [ ] **Step 1: Write the failing tests**

Append to `src-tauri/tests/sources.rs`:

```rust
fn write_file(dir: &std::path::Path, name: &str, data: &[u8]) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, data).unwrap();
    path
}

#[test]
fn zip_lists_files_and_reads_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "book.cbz",
        &zip_bytes(&[("p1.png", png(1, 2)), ("ch1/p2.png", png(3, 4))]),
    );
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(sorted_list(source.as_ref()), ["ch1/p2.png", "p1.png"]);
    assert_eq!(source.read("ch1/p2.png").unwrap(), png(3, 4));
    assert_eq!(source.read_prefix("p1.png", 4).unwrap(), png(1, 2)[..4].to_vec());
    assert!(matches!(source.read("missing.png"), Err(SourceError::NotFound(_))));
}

#[test]
fn zip_backslash_names_become_folders() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "win.zip",
        &zip_bytes(&[("ch1\\p2.png", png(3, 4)), (".\\p1.png", png(1, 2))]),
    );
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(sorted_list(source.as_ref()), ["ch1/p2.png", "p1.png"]);
    assert_eq!(source.read("ch1/p2.png").unwrap(), png(3, 4));
}

#[test]
fn zip_with_wrong_extension_opens_as_zip() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "mislabeled.cbr", &zip_bytes(&[("p1.png", png(1, 2))]));
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(source.list().unwrap(), ["p1.png"]);
}

#[test]
fn zip_encrypted_is_reported() {
    use std::io::Write;
    use zip::unstable::write::FileOptionsExt;
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .with_deprecated_encryption(b"secret")
        .unwrap();
    w.start_file("p1.png", options).unwrap();
    w.write_all(&png(1, 2)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "locked.cbz", &w.finish().unwrap().into_inner());
    let base = temp_base();
    assert!(matches!(open_source(&path, base.path()), Err(SourceError::Encrypted)));
}

#[test]
fn zip_damaged_is_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let mut data = zip_bytes(&[("p1.png", png(1, 2))]);
    data.truncate(data.len() - 30); // cut into the central directory
    let path = write_file(dir.path(), "broken.cbz", &data);
    let base = temp_base();
    assert!(matches!(open_source(&path, base.path()), Err(SourceError::Corrupt(_))));
}
```

`open_source` returns `Box<dyn PageSource>`, which is not `Debug`, so the tests use `matches!` and never `unwrap_err`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test sources zip_`
Expected: FAIL — `open_source` returns `Err(Unsupported)` for zip files.

- [ ] **Step 3: Implement `source/zip_source.rs`**

```rust
use super::{PageSource, SourceError};
use crate::image_entry::normalize_entry_name;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;
use zip::result::ZipError;
use zip::ZipArchive;

pub struct ZipSource {
    archive: Mutex<ZipArchive<File>>,
    names: Vec<String>,
    index: HashMap<String, usize>,
}

fn map_zip_error(err: ZipError) -> SourceError {
    match err {
        ZipError::Io(err) => SourceError::Io(err),
        ZipError::InvalidPassword => SourceError::Encrypted,
        ZipError::UnsupportedArchive(msg) if msg == ZipError::PASSWORD_REQUIRED => {
            SourceError::Encrypted
        }
        other => SourceError::Corrupt(other.to_string()),
    }
}

impl ZipSource {
    pub fn open(path: &Path) -> Result<Self, SourceError> {
        let mut archive = ZipArchive::new(File::open(path)?).map_err(map_zip_error)?;
        let mut names = Vec::new();
        let mut index = HashMap::new();
        for i in 0..archive.len() {
            // by_index fails with PASSWORD_REQUIRED for encrypted entries.
            let entry = archive.by_index(i).map_err(map_zip_error)?;
            if entry.is_dir() {
                continue;
            }
            let name = normalize_entry_name(entry.name());
            if name.is_empty() {
                continue;
            }
            index.insert(name.clone(), i);
            names.push(name);
        }
        Ok(Self { archive: Mutex::new(archive), names, index })
    }

    fn read_limited(&self, path: &str, limit: Option<usize>) -> Result<Vec<u8>, SourceError> {
        let i = *self.index.get(path).ok_or_else(|| SourceError::NotFound(path.to_owned()))?;
        let mut archive = self.archive.lock().unwrap();
        let entry = archive.by_index(i).map_err(map_zip_error)?;
        let mut buf = Vec::new();
        match limit {
            Some(n) => entry.take(n as u64).read_to_end(&mut buf)?,
            None => { let mut entry = entry; entry.read_to_end(&mut buf)? }
        };
        Ok(buf)
    }
}

impl PageSource for ZipSource {
    fn list(&self) -> Result<Vec<String>, SourceError> {
        Ok(self.names.clone())
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, SourceError> {
        self.read_limited(path, None)
    }

    fn read_prefix(&self, path: &str, limit: usize) -> Result<Vec<u8>, SourceError> {
        self.read_limited(path, Some(limit))
    }
}
```

- [ ] **Step 4: Wire it into `source/mod.rs`**

Add the module and export next to `mod folder;`:

```rust
mod zip_source;

pub use zip_source::ZipSource;
```

Change the `open_source` match:

```rust
pub fn open_source(path: &Path, _temp_base: &Path) -> Result<Box<dyn PageSource>, SourceError> {
    match detect(path)? {
        SourceKind::Folder => Ok(Box::new(FolderSource::new(path))),
        SourceKind::Zip => Ok(Box::new(ZipSource::open(path)?)),
        _ => Err(SourceError::Unsupported),
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test sources`
Expected: all 11 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/source src-tauri/tests/sources.rs
git commit -m "feat: add zip page source

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Per-instance temporary folder

**Files:**
- Create: `src-tauri/src/temp.rs`
- Modify: `src-tauri/src/lib.rs` (add module)

**Interfaces:**
- Produces (module `comic_reader_lib::temp`):
  - `pub struct InstanceDir` with `pub fn create(base: &Path) -> io::Result<InstanceDir>`, `pub fn path(&self) -> &Path`, `pub fn remove(self)`.
  - `pub fn cleanup_stale(base: &Path)` — deletes `inst-*` folders whose `.lock` file is not locked by a running process.
  - `pub fn default_base() -> PathBuf` — `<system temp>/comic-reader`.

Each running app holds an exclusive lock on `<instance>/.lock`. The OS releases the lock when the process ends, also after a crash, so `cleanup_stale` never deletes the folder of a running instance.

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/src/temp.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_makes_locked_folder_under_base() {
        let base = tempfile::tempdir().unwrap();
        let inst = InstanceDir::create(base.path()).unwrap();
        assert!(inst.path().starts_with(base.path()));
        assert!(inst.path().join(LOCK_FILE).is_file());
        assert!(!is_unlocked(inst.path()));
    }

    #[test]
    fn cleanup_keeps_live_and_removes_stale() {
        let base = tempfile::tempdir().unwrap();
        let live = InstanceDir::create(base.path()).unwrap();

        let stale = base.path().join("inst-stale");
        fs::create_dir_all(stale.join("book-1")).unwrap();
        fs::write(stale.join(LOCK_FILE), b"").unwrap();
        fs::write(stale.join("book-1/p1.png"), b"x").unwrap();

        let no_lock = base.path().join("inst-nolock");
        fs::create_dir_all(&no_lock).unwrap();

        let other = base.path().join("unrelated");
        fs::create_dir_all(&other).unwrap();

        cleanup_stale(base.path());

        assert!(live.path().is_dir());
        assert!(!stale.exists());
        assert!(!no_lock.exists());
        assert!(other.is_dir());
    }

    #[test]
    fn remove_deletes_folder() {
        let base = tempfile::tempdir().unwrap();
        let inst = InstanceDir::create(base.path()).unwrap();
        let path = inst.path().to_path_buf();
        fs::write(path.join("x"), b"x").unwrap();
        inst.remove();
        assert!(!path.exists());
    }

    #[test]
    fn cleanup_of_missing_base_does_nothing() {
        let base = tempfile::tempdir().unwrap();
        cleanup_stale(&base.path().join("does-not-exist"));
    }
}
```

Add to `src-tauri/src/lib.rs`:

```rust
pub mod temp;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib temp`
Expected: compile errors for the missing items.

- [ ] **Step 3: Write the implementation**

Insert above the test module:

```rust
//! One temporary folder per running app instance, protected by a file lock.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

const PREFIX: &str = "inst-";
const LOCK_FILE: &str = ".lock";

pub struct InstanceDir {
    path: PathBuf,
    lock: Option<File>,
}

impl InstanceDir {
    pub fn create(base: &Path) -> io::Result<Self> {
        fs::create_dir_all(base)?;
        let path = tempfile::Builder::new().prefix(PREFIX).tempdir_in(base)?.keep();
        let lock = File::create(path.join(LOCK_FILE))?;
        lock.try_lock().map_err(|e| io::Error::other(e.to_string()))?;
        Ok(Self { path, lock: Some(lock) })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Releases the lock and deletes the folder. Windows cannot delete an open
    /// file, so the lock file is closed first.
    pub fn remove(mut self) {
        drop(self.lock.take());
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn default_base() -> PathBuf {
    std::env::temp_dir().join("comic-reader")
}

fn is_unlocked(dir: &Path) -> bool {
    match OpenOptions::new().write(true).open(dir.join(LOCK_FILE)) {
        Err(_) => true,
        Ok(file) => file.try_lock().is_ok(),
    }
}

/// Deletes instance folders left behind by app instances that are no longer running.
pub fn cleanup_stale(base: &Path) {
    let Ok(entries) = fs::read_dir(base) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_instance = entry.file_name().to_string_lossy().starts_with(PREFIX) && path.is_dir();
        if is_instance && is_unlocked(&path) {
            let _ = fs::remove_dir_all(&path);
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib temp`
Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/temp.rs src-tauri/src/lib.rs
git commit -m "feat: add locked per-instance temp folder with stale cleanup

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: tar.gz extraction

**Files:**
- Create: `src-tauri/src/source/extract.rs`
- Modify: `src-tauri/src/source/mod.rs`, `src-tauri/tests/sources.rs`

**Interfaces:**
- Consumes: `FolderSource`, `safe_relative_path`, `SourceError` (Task 4); `is_page_entry`, `normalize_entry_name` (Task 3).
- Produces: `pub struct ExtractedSource` with `pub fn from_tar_gz(archive: &Path, temp_base: &Path) -> Result<ExtractedSource, SourceError>`. It extracts only page entries into a `book-*` folder under `temp_base`; the folder is deleted when the `ExtractedSource` is dropped. Entries with unsafe paths are skipped and never written.

- [ ] **Step 1: Write the failing tests**

Append to `src-tauri/tests/sources.rs`:

```rust
#[test]
fn tar_gz_extracts_only_pages() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "book.tar.gz",
        &tar_gz_bytes(&[
            ("ch1/p2.png", png(3, 4)),
            ("p1.png", png(1, 2)),
            ("notes.txt", b"not a page".to_vec()),
        ]),
    );
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(sorted_list(source.as_ref()), ["ch1/p2.png", "p1.png"]);
    assert_eq!(source.read("ch1/p2.png").unwrap(), png(3, 4));
}

#[test]
fn tar_gz_from_macos_tar_skips_appledouble_and_dot_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "mac.tgz",
        &tar_gz_bytes(&[
            ("./p1.png", png(1, 2)),
            ("./._p1.png", b"appledouble".to_vec()),
            ("./ch1/p2.png", png(3, 4)),
        ]),
    );
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(sorted_list(source.as_ref()), ["ch1/p2.png", "p1.png"]);
}

#[test]
fn tar_gz_never_writes_outside_its_folder() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(
        dir.path(),
        "evil.tgz",
        &tar_gz_bytes(&[("../evil.png", png(1, 1)), ("ok.png", png(2, 2))]),
    );
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(source.list().unwrap(), ["ok.png"]);
    assert!(!base.path().join("evil.png").exists());
    // The archive's own folder is book-*/ under base, so "../evil.png" would land in base.
    for entry in std::fs::read_dir(base.path()).unwrap() {
        assert_ne!(entry.unwrap().file_name(), "evil.png");
    }
}

#[test]
fn tar_gz_folder_is_deleted_on_drop() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "book.tgz", &tar_gz_bytes(&[("p1.png", png(1, 2))]));
    let base = temp_base();
    let source = open_source(&path, base.path()).unwrap();
    assert_eq!(std::fs::read_dir(base.path()).unwrap().count(), 1);
    drop(source);
    assert_eq!(std::fs::read_dir(base.path()).unwrap().count(), 0);
}

#[test]
fn damaged_tar_gz_is_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let mut data = tar_gz_bytes(&[("p1.png", vec![7u8; 4000])]);
    data.truncate(data.len() / 2);
    let path = write_file(dir.path(), "broken.tgz", &data);
    let base = temp_base();
    assert!(matches!(open_source(&path, base.path()), Err(SourceError::Corrupt(_))));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test sources tar_gz`
Expected: FAIL — `open_source` returns `Err(Unsupported)`.

- [ ] **Step 3: Implement `source/extract.rs`**

```rust
use super::{safe_relative_path, FolderSource, PageSource, SourceError};
use crate::image_entry::{is_page_entry, normalize_entry_name};
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::Path;
use tempfile::TempDir;

/// An archive extracted to a temporary folder. The folder is deleted on drop.
pub struct ExtractedSource {
    _dir: TempDir,
    inner: FolderSource,
}

impl ExtractedSource {
    pub fn from_tar_gz(archive: &Path, temp_base: &Path) -> Result<Self, SourceError> {
        let dir = new_book_dir(temp_base)?;
        extract_tar_gz(archive, dir.path())?;
        Ok(Self::wrap(dir))
    }

    fn wrap(dir: TempDir) -> Self {
        let inner = FolderSource::new(dir.path());
        Self { _dir: dir, inner }
    }
}

impl PageSource for ExtractedSource {
    fn list(&self) -> Result<Vec<String>, SourceError> {
        self.inner.list()
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, SourceError> {
        self.inner.read(path)
    }

    fn read_prefix(&self, path: &str, limit: usize) -> Result<Vec<u8>, SourceError> {
        self.inner.read_prefix(path, limit)
    }
}

fn new_book_dir(temp_base: &Path) -> io::Result<TempDir> {
    fs::create_dir_all(temp_base)?;
    tempfile::Builder::new().prefix("book-").tempdir_in(temp_base)
}

/// Where `name` should be written, or `None` if it is not a page or not safe.
fn target_for(dest: &Path, name: &str) -> Option<std::path::PathBuf> {
    if !is_page_entry(&normalize_entry_name(name)) {
        return None;
    }
    safe_relative_path(name).map(|rel| dest.join(rel))
}

fn write_reader(target: &Path, reader: &mut impl Read) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    io::copy(reader, &mut File::create(target)?)?;
    Ok(())
}

fn tar_error(err: io::Error) -> SourceError {
    match err.kind() {
        io::ErrorKind::InvalidData | io::ErrorKind::InvalidInput | io::ErrorKind::UnexpectedEof => {
            SourceError::Corrupt(err.to_string())
        }
        _ => SourceError::Io(err),
    }
}

fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), SourceError> {
    let file = File::open(archive)?;
    let mut tar = tar::Archive::new(GzDecoder::new(BufReader::new(file)));
    for entry in tar.entries().map_err(tar_error)? {
        let mut entry = entry.map_err(tar_error)?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        if let Some(target) = target_for(dest, &name) {
            write_reader(&target, &mut entry).map_err(tar_error)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Wire it into `source/mod.rs`**

Add next to the other modules:

```rust
mod extract;

pub use extract::ExtractedSource;
```

Replace `open_source`:

```rust
pub fn open_source(path: &Path, temp_base: &Path) -> Result<Box<dyn PageSource>, SourceError> {
    match detect(path)? {
        SourceKind::Folder => Ok(Box::new(FolderSource::new(path))),
        SourceKind::Zip => Ok(Box::new(ZipSource::open(path)?)),
        SourceKind::TarGz => Ok(Box::new(ExtractedSource::from_tar_gz(path, temp_base)?)),
        SourceKind::Rar => Err(SourceError::Unsupported),
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test sources`
Expected: all 16 tests PASS. If `damaged_tar_gz_is_corrupt` gets `Io` instead of `Corrupt`, print the error kind and add that kind to `tar_error`; do not weaken the test to accept `Io`.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/source src-tauri/tests/sources.rs
git commit -m "feat: extract tar.gz pages to a temporary folder

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: RAR fixtures and extraction

**Files:**
- Create: `scripts/make-rar-fixtures.sh`, `src-tauri/tests/fixtures/book.rar`, `book.cbr`, `encrypted-files.rar`, `encrypted-headers.rar`
- Modify: `src-tauri/src/source/extract.rs`, `src-tauri/src/source/mod.rs`, `src-tauri/tests/sources.rs`

**Interfaces:**
- Consumes: `ExtractedSource`, `target_for`, `new_book_dir` (Task 7).
- Produces: `ExtractedSource::from_rar(archive: &Path, temp_base: &Path) -> Result<ExtractedSource, SourceError>`; `open_source` handles `SourceKind::Rar`.
- Fixture content (all four archives): `ch2/page 10.png` (30×40), `ch2/page 2.png` (20×30), `ch10/page 1.png` (10×20), `notes.txt`. `book.rar` is solid. `encrypted-files.rar` uses `-p`, `encrypted-headers.rar` uses `-hp`, both with password `Secret`.

- [ ] **Step 1: Install the rar CLI**

Run: `brew install --cask rar`
Expected: `rar` prints its version banner when run with no arguments.

- [ ] **Step 2: Create `scripts/make-rar-fixtures.sh`**

```bash
#!/usr/bin/env bash
# Creates the RAR test fixtures. Requires the rar CLI (macOS: brew install --cask rar).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/src-tauri/tests/fixtures"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

python3 - "$WORK" <<'PY'
import pathlib, struct, sys, zlib

def png(w, h, rgb):
    raw = b"".join(b"\x00" + bytes(rgb) * w for _ in range(h))
    def chunk(kind, data):
        crc = zlib.crc32(kind + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", crc)
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")

root = pathlib.Path(sys.argv[1]) / "book"
(root / "ch2").mkdir(parents=True)
(root / "ch10").mkdir()
(root / "ch2" / "page 10.png").write_bytes(png(30, 40, (200, 0, 0)))
(root / "ch2" / "page 2.png").write_bytes(png(20, 30, (0, 200, 0)))
(root / "ch10" / "page 1.png").write_bytes(png(10, 20, (0, 0, 200)))
(root / "notes.txt").write_text("not an image\n")
PY

mkdir -p "$OUT"
rm -f "$OUT"/*.rar "$OUT"/*.cbr
cd "$WORK/book"
rar a -r -s -idq "$OUT/book.rar" .
cp "$OUT/book.rar" "$OUT/book.cbr"
rar a -r -idq -pSecret "$OUT/encrypted-files.rar" .
rar a -r -idq -hpSecret "$OUT/encrypted-headers.rar" .
ls -l "$OUT"
```

- [ ] **Step 3: Create the fixtures**

Run: `chmod +x scripts/make-rar-fixtures.sh && scripts/make-rar-fixtures.sh`
Expected: `src-tauri/tests/fixtures/` lists `book.rar`, `book.cbr`, `encrypted-files.rar`, `encrypted-headers.rar`, each a few hundred bytes.

- [ ] **Step 4: Write the failing tests**

Append to `src-tauri/tests/sources.rs`:

```rust
#[test]
fn rar_extracts_only_pages() {
    for name in ["book.rar", "book.cbr"] {
        let base = temp_base();
        let source = open_source(&fixture(name), base.path()).unwrap();
        assert_eq!(
            sorted_list(source.as_ref()),
            ["ch10/page 1.png", "ch2/page 10.png", "ch2/page 2.png"],
            "{name}"
        );
        let bytes = source.read("ch2/page 2.png").unwrap();
        assert!(bytes.starts_with(b"\x89PNG"), "{name}");
    }
}

#[test]
fn rar_encrypted_is_reported() {
    for name in ["encrypted-files.rar", "encrypted-headers.rar"] {
        let base = temp_base();
        assert!(
            matches!(open_source(&fixture(name), base.path()), Err(SourceError::Encrypted)),
            "{name}"
        );
    }
}

#[test]
fn rar_damaged_is_corrupt_or_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    let mut data = std::fs::read(fixture("book.rar")).unwrap();
    data.truncate(data.len() / 2);
    let path = write_file(dir.path(), "broken.rar", &data);
    let base = temp_base();
    // UnRAR may report bad data, or may stop at the cut as if the archive ended.
    // Either is acceptable; a crash, an I/O error, or all 3 pages is not.
    match open_source(&path, base.path()) {
        Err(SourceError::Corrupt(_)) => {}
        Ok(source) => assert!(source.list().unwrap().len() < 3),
        Err(other) => panic!("expected Corrupt, got {other}"),
    }
}
```

- [ ] **Step 5: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test sources rar_`
Expected: FAIL — `open_source` returns `Err(Unsupported)`.

- [ ] **Step 6: Implement RAR extraction**

Add to `src-tauri/src/source/extract.rs` inside `impl ExtractedSource`:

```rust
    pub fn from_rar(archive: &Path, temp_base: &Path) -> Result<Self, SourceError> {
        let dir = new_book_dir(temp_base)?;
        extract_rar(archive, dir.path())?;
        Ok(Self::wrap(dir))
    }
```

Add at the end of the file:

```rust
fn rar_error(err: unrar::error::UnrarError) -> SourceError {
    use unrar::error::Code;
    match err.code {
        Code::MissingPassword | Code::BadPassword => SourceError::Encrypted,
        Code::EOpen | Code::ECreate | Code::EWrite | Code::ERead | Code::EClose => {
            SourceError::Io(io::Error::other(err.to_string()))
        }
        _ => SourceError::Corrupt(err.to_string()),
    }
}

/// RAR has no random access (solid archives), so entries are processed front to back.
fn extract_rar(archive: &Path, dest: &Path) -> Result<(), SourceError> {
    let mut cursor = unrar::Archive::new(archive).open_for_processing().map_err(rar_error)?;
    while let Some(header) = cursor.read_header().map_err(rar_error)? {
        let entry = header.entry();
        let target = if entry.is_file() {
            target_for(dest, &entry.filename.to_string_lossy())
        } else {
            None
        };
        cursor = match target {
            Some(target) => {
                let (data, next) = header.read().map_err(rar_error)?;
                write_reader(&target, &mut data.as_slice())?;
                next
            }
            None => header.skip().map_err(rar_error)?,
        };
    }
    Ok(())
}
```

In `source/mod.rs`, change the RAR arm of `open_source`:

```rust
        SourceKind::Rar => Ok(Box::new(ExtractedSource::from_rar(path, temp_base)?)),
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test sources`
Expected: all 19 tests PASS. `encrypted-files.rar` fails at `read()` and `encrypted-headers.rar` fails at `read_header()`; both report `Code::MissingPassword` (this matches the `unrar` crate's own tests).

- [ ] **Step 8: Commit**

```bash
git add scripts/make-rar-fixtures.sh src-tauri/tests/fixtures src-tauri/src/source src-tauri/tests/sources.rs
git commit -m "feat: extract rar pages to a temporary folder

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: Book

**Files:**
- Create: `src-tauri/src/book.rs`, `src-tauri/tests/book.rs`
- Modify: `src-tauri/src/lib.rs` (add module)

**Interfaces:**
- Consumes: `open_source`, `PageSource`, `SourceError` (Tasks 4–8); `is_page_entry`, `page_size` (Task 3); `natural_cmp` (Task 2).
- Produces (module `comic_reader_lib::book`):
  - `#[derive(Serialize)] pub struct PageInfo { pub name: String, pub width: Option<u32>, pub height: Option<u32> }`
  - `#[derive(Serialize)] #[serde(rename_all = "camelCase")] pub struct BookInfo { pub book_id: u64, pub title: String, pub pages: Vec<PageInfo> }` — JSON keys `bookId`, `title`, `pages[].name/width/height`.
  - `pub enum OpenError { Source(SourceError), NoImages }` with `pub fn message(&self, name: &str) -> String`.
  - `pub struct Book { pub id: u64, pub title: String, .. }` with `pub fn open(path: &Path, id: u64, temp_base: &Path) -> Result<Book, OpenError>`, `pub fn info(&self) -> BookInfo`, `pub fn page_count(&self) -> usize`, `pub fn page_name(&self, index: usize) -> Option<&str>`, `pub fn read_page(&self, index: usize) -> Result<Vec<u8>, SourceError>`.
  - `pub fn title_from_path(path: &Path) -> String`.

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/tests/book.rs`:

```rust
mod common;

use comic_reader_lib::book::{title_from_path, Book, OpenError};
use comic_reader_lib::source::SourceError;
use common::*;
use std::path::Path;

fn names(book: &Book) -> Vec<String> {
    book.info().pages.into_iter().map(|p| p.name).collect()
}

#[test]
fn folder_book_is_filtered_and_sorted() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(
        dir.path(),
        &[
            ("page 10.png", png(10, 10)),
            ("page 2.png", png(2, 2)),
            ("page 1.png", png(1, 1)),
            ("notes.txt", b"x".to_vec()),
            ("._page 1.png", b"appledouble".to_vec()),
            ("__MACOSX/page 3.png", png(3, 3)),
        ],
    );
    let base = temp_base();
    let book = Book::open(dir.path(), 7, base.path()).unwrap();
    assert_eq!(book.id, 7);
    assert_eq!(names(&book), ["page 1.png", "page 2.png", "page 10.png"]);
    assert_eq!(book.page_count(), 3);
    assert_eq!(book.page_name(2), Some("page 10.png"));
    assert_eq!(book.page_name(3), None);
    assert_eq!(book.read_page(1).unwrap(), png(2, 2));
}

#[test]
fn page_sizes_come_from_headers() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("a.png", png(800, 1200))]);
    let base = temp_base();
    let info = Book::open(dir.path(), 1, base.path()).unwrap().info();
    assert_eq!(info.pages[0].width, Some(800));
    assert_eq!(info.pages[0].height, Some(1200));
}

#[test]
fn empty_and_truncated_pages_have_unknown_size_but_book_opens() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("1.png", Vec::new()), ("2.png", png(5, 5)[..10].to_vec()), ("3.png", png(4, 6))]);
    let base = temp_base();
    let book = Book::open(dir.path(), 1, base.path()).unwrap();
    let info = book.info();
    assert_eq!((info.pages[0].width, info.pages[0].height), (None, None));
    assert_eq!((info.pages[1].width, info.pages[1].height), (None, None));
    assert_eq!((info.pages[2].width, info.pages[2].height), (Some(4), Some(6)));
    assert_eq!(book.read_page(0).unwrap(), Vec::<u8>::new());
}

#[test]
fn rar_book_sorts_across_folders() {
    let base = temp_base();
    let book = Book::open(&fixture("book.cbr"), 1, base.path()).unwrap();
    assert_eq!(names(&book), ["ch2/page 2.png", "ch2/page 10.png", "ch10/page 1.png"]);
    let info = book.info();
    assert_eq!((info.pages[0].width, info.pages[0].height), (Some(20), Some(30)));
    assert_eq!(info.title, "book");
}

#[test]
fn zip_book_reads_sizes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Vol 1.CBZ");
    std::fs::write(&path, zip_bytes(&[("p2.png", png(2, 3)), ("p1.png", png(1, 2))])).unwrap();
    let base = temp_base();
    let info = Book::open(&path, 1, base.path()).unwrap().info();
    assert_eq!(info.title, "Vol 1");
    assert_eq!(info.pages[0].name, "p1.png");
    assert_eq!((info.pages[0].width, info.pages[0].height), (Some(1), Some(2)));
}

#[test]
fn source_without_images_is_no_images() {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), &[("notes.txt", b"x".to_vec())]);
    let base = temp_base();
    assert!(matches!(Book::open(dir.path(), 1, base.path()), Err(OpenError::NoImages)));
}

#[test]
fn plain_gzip_is_an_error_not_a_crash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("single.gz");
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut enc, &[b'x'; 3000]).unwrap();
    std::fs::write(&path, enc.finish().unwrap()).unwrap();
    let base = temp_base();
    assert!(Book::open(&path, 1, base.path()).is_err());
}

#[test]
fn error_messages_name_the_source() {
    let name = "Vol 1.cbz";
    assert_eq!(OpenError::NoImages.message(name), "No images found in Vol 1.cbz.");
    assert_eq!(
        OpenError::Source(SourceError::Encrypted).message(name),
        "Vol 1.cbz is password protected. Password-protected archives are not supported."
    );
    assert_eq!(
        OpenError::Source(SourceError::Corrupt("x".into())).message(name),
        "Vol 1.cbz is damaged or is not a valid archive."
    );
    assert_eq!(
        OpenError::Source(SourceError::Unsupported).message(name),
        "Vol 1.cbz is not a supported comic file or folder."
    );
    let full = std::io::Error::from(std::io::ErrorKind::StorageFull);
    assert_eq!(
        OpenError::Source(SourceError::Io(full)).message(name),
        "Not enough disk space to open Vol 1.cbz."
    );
}

#[test]
fn titles_strip_archive_extensions() {
    assert_eq!(title_from_path(Path::new("/a/Vol 1.tar.gz")), "Vol 1");
    assert_eq!(title_from_path(Path::new("/a/Vol 1.TGZ")), "Vol 1");
    assert_eq!(title_from_path(Path::new("/a/One.Piece.cbr")), "One.Piece");
    assert_eq!(title_from_path(Path::new("/a/notes.txt")), "notes.txt");
}
```

Add to `src-tauri/src/lib.rs`:

```rust
pub mod book;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test book`
Expected: compile errors for the missing module items.

- [ ] **Step 3: Implement `src-tauri/src/book.rs`**

```rust
//! A book: the sorted, sized pages of one source.

use crate::image_entry::{is_page_entry, page_size};
use crate::natural_sort::natural_cmp;
use crate::source::{open_source, PageSource, SourceError};
use serde::Serialize;
use std::io;
use std::path::Path;

/// Header bytes read first, then once more if the size is not found (large EXIF blocks).
const PROBE_LIMITS: [usize; 2] = [64 * 1024, 1024 * 1024];
const ARCHIVE_EXTENSIONS: [&str; 6] = [".tar.gz", ".tgz", ".zip", ".cbz", ".rar", ".cbr"];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PageInfo {
    pub name: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookInfo {
    pub book_id: u64,
    pub title: String,
    pub pages: Vec<PageInfo>,
}

#[derive(Debug)]
pub enum OpenError {
    Source(SourceError),
    NoImages,
}

impl OpenError {
    /// A message for the user. `name` is the file or folder name.
    pub fn message(&self, name: &str) -> String {
        match self {
            OpenError::NoImages => format!("No images found in {name}."),
            OpenError::Source(SourceError::Encrypted) => format!(
                "{name} is password protected. Password-protected archives are not supported."
            ),
            OpenError::Source(SourceError::Corrupt(_)) => {
                format!("{name} is damaged or is not a valid archive.")
            }
            OpenError::Source(SourceError::Unsupported) => {
                format!("{name} is not a supported comic file or folder.")
            }
            OpenError::Source(SourceError::Io(err)) if err.kind() == io::ErrorKind::StorageFull => {
                format!("Not enough disk space to open {name}.")
            }
            OpenError::Source(SourceError::Io(err)) if err.kind() == io::ErrorKind::NotFound => {
                format!("{name} was not found.")
            }
            OpenError::Source(err) => format!("Could not open {name}: {err}."),
        }
    }
}

pub struct Book {
    pub id: u64,
    pub title: String,
    pages: Vec<PageInfo>,
    source: Box<dyn PageSource>,
}

impl Book {
    pub fn open(path: &Path, id: u64, temp_base: &Path) -> Result<Book, OpenError> {
        let source = open_source(path, temp_base).map_err(OpenError::Source)?;
        let mut names: Vec<String> = source
            .list()
            .map_err(OpenError::Source)?
            .into_iter()
            .filter(|name| is_page_entry(name))
            .collect();
        if names.is_empty() {
            return Err(OpenError::NoImages);
        }
        names.sort_by(|a, b| natural_cmp(a, b));
        let pages = names
            .into_iter()
            .map(|name| {
                let size = probe_size(source.as_ref(), &name);
                PageInfo { width: size.map(|s| s.0), height: size.map(|s| s.1), name }
            })
            .collect();
        Ok(Book { id, title: title_from_path(path), pages, source })
    }

    pub fn info(&self) -> BookInfo {
        BookInfo { book_id: self.id, title: self.title.clone(), pages: self.pages.clone() }
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn page_name(&self, index: usize) -> Option<&str> {
        self.pages.get(index).map(|p| p.name.as_str())
    }

    pub fn read_page(&self, index: usize) -> Result<Vec<u8>, SourceError> {
        let name = self.page_name(index).ok_or_else(|| SourceError::NotFound(index.to_string()))?;
        self.source.read(name)
    }
}

fn probe_size(source: &dyn PageSource, name: &str) -> Option<(u32, u32)> {
    for limit in PROBE_LIMITS {
        let bytes = source.read_prefix(name, limit).ok()?;
        if let Some(size) = page_size(&bytes) {
            return Some(size);
        }
        if bytes.len() < limit {
            return None; // whole file read; a larger limit cannot help
        }
    }
    None
}

pub fn title_from_path(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    if path.is_dir() {
        return name;
    }
    for ext in ARCHIVE_EXTENSIONS {
        let cut = name.len().saturating_sub(ext.len());
        if name.is_char_boundary(cut) && name[cut..].eq_ignore_ascii_case(ext) && cut > 0 {
            return name[..cut].to_owned();
        }
    }
    name
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test book`
Expected: 9 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/book.rs src-tauri/src/lib.rs src-tauri/tests/book.rs
git commit -m "feat: open a book with filtered, sorted, sized pages

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 10: App state, page protocol, and commands

**Files:**
- Create: `src-tauri/src/state.rs`, `src-tauri/src/protocol.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/launch_open.rs`, `src-tauri/tests/app.rs`
- Modify: `src-tauri/src/lib.rs` (full replacement)

**Interfaces:**
- Consumes: `Book`, `BookInfo`, `OpenError` (Task 9); `InstanceDir`, `cleanup_stale`, `default_base` (Task 6); `content_type` (Task 3).
- Produces:
  - `comic_reader_lib::state::AppState` with `pub fn new(instance: InstanceDir) -> Self`, `pub fn temp_base(&self) -> &Path`, `pub fn next_ticket(&self) -> u64`, `pub fn is_latest(&self, ticket: u64) -> bool`, `pub fn install(&self, book: Book) -> bool`, `pub fn current(&self) -> Option<Arc<Book>>`, `pub fn shutdown(&self)`.
  - `comic_reader_lib::protocol::parse_page_path(path: &str) -> Option<(u64, usize)>` and `page_response(book: Option<&Book>, path: &str) -> tauri::http::Response<Vec<u8>>`.
  - Tauri commands `open_book(path: String) -> Result<BookInfo, String>` and `take_pending_open() -> Option<String>`. The error string `"superseded"` means a newer open replaced this one; the frontend ignores it.
  - `comic_reader_lib::launch_open::{PendingOpen, OPEN_EVENT}` where `OPEN_EVENT = "open-path"`.

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/tests/app.rs`:

```rust
mod common;

use comic_reader_lib::book::Book;
use comic_reader_lib::protocol::{page_response, parse_page_path};
use comic_reader_lib::state::AppState;
use comic_reader_lib::temp::InstanceDir;
use common::*;

fn folder_with(files: &[(&str, Vec<u8>)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write_folder(dir.path(), files);
    dir
}

#[test]
fn parses_plain_and_encoded_paths() {
    assert_eq!(parse_page_path("/3/17"), Some((3, 17)));
    assert_eq!(parse_page_path("/3%2F17"), Some((3, 17)));
    assert_eq!(parse_page_path("/3%2f17"), Some((3, 17)));
    assert_eq!(parse_page_path("/x/1"), None);
    assert_eq!(parse_page_path("/3"), None);
    assert_eq!(parse_page_path("/3/-1"), None);
    assert_eq!(parse_page_path("/3/1/../../etc"), None);
}

#[test]
fn serves_pages_with_content_type() {
    let dir = folder_with(&[("b.png", png(2, 2)), ("a.jpg", b"jpegbytes".to_vec())]);
    let base = temp_base();
    let book = Book::open(dir.path(), 5, base.path()).unwrap();

    let res = page_response(Some(&book), "/5/0");
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["Content-Type"], "image/jpeg");
    assert_eq!(res.body(), b"jpegbytes");

    let res = page_response(Some(&book), "/5%2F1");
    assert_eq!(res.headers()["Content-Type"], "image/png");
}

#[test]
fn rejects_wrong_book_bad_index_and_no_book() {
    let dir = folder_with(&[("a.png", png(1, 1))]);
    let base = temp_base();
    let book = Book::open(dir.path(), 5, base.path()).unwrap();
    assert_eq!(page_response(Some(&book), "/4/0").status(), 404);
    assert_eq!(page_response(Some(&book), "/5/1").status(), 404);
    assert_eq!(page_response(None, "/5/0").status(), 404);
    assert_eq!(page_response(Some(&book), "/nonsense").status(), 400);
}

#[test]
fn page_deleted_after_open_is_server_error() {
    let dir = folder_with(&[("a.png", png(1, 1))]);
    let base = temp_base();
    let book = Book::open(dir.path(), 1, base.path()).unwrap();
    std::fs::remove_file(dir.path().join("a.png")).unwrap();
    assert_eq!(page_response(Some(&book), "/1/0").status(), 500);
}

#[test]
fn latest_open_wins_and_old_book_id_is_rejected() {
    let base = temp_base();
    let state = AppState::new(InstanceDir::create(base.path()).unwrap());
    let first = folder_with(&[("a.png", png(1, 1))]);
    let second = folder_with(&[("b.png", png(2, 2))]);

    let t1 = state.next_ticket();
    assert!(state.install(Book::open(first.path(), t1, state.temp_base()).unwrap()));
    assert_eq!(state.current().unwrap().id, t1);

    let t2 = state.next_ticket();
    let t3 = state.next_ticket();
    assert!(!state.is_latest(t2));
    // t2 finishes after t3 was requested: discarded.
    assert!(!state.install(Book::open(first.path(), t2, state.temp_base()).unwrap()));
    assert!(state.install(Book::open(second.path(), t3, state.temp_base()).unwrap()));

    let current = state.current().unwrap();
    assert_eq!(current.id, t3);
    // A late request for the first book's page must not return the new book's page.
    assert_eq!(page_response(Some(current.as_ref()), &format!("/{t1}/0")).status(), 404);
    assert_eq!(page_response(Some(current.as_ref()), &format!("/{t3}/0")).body(), &png(2, 2));
}

#[test]
fn shutdown_clears_book_and_removes_temp_folder() {
    let base = temp_base();
    let state = AppState::new(InstanceDir::create(base.path()).unwrap());
    let inst_path = state.temp_base().to_path_buf();
    let archive_dir = tempfile::tempdir().unwrap();
    let archive = archive_dir.path().join("b.tgz");
    std::fs::write(&archive, tar_gz_bytes(&[("p1.png", png(1, 1))])).unwrap();

    let t = state.next_ticket();
    assert!(state.install(Book::open(&archive, t, state.temp_base()).unwrap()));
    state.shutdown();
    assert!(state.current().is_none());
    assert!(!inst_path.exists());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test app`
Expected: compile errors for the missing `state` and `protocol` modules.

- [ ] **Step 3: Implement `src-tauri/src/state.rs`**

```rust
//! Shared app state: the current book and the open-request tickets.

use crate::book::Book;
use crate::temp::InstanceDir;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct AppState {
    temp_base: PathBuf,
    instance: Mutex<Option<InstanceDir>>,
    current: Mutex<Option<Arc<Book>>>,
    latest: AtomicU64,
}

impl AppState {
    pub fn new(instance: InstanceDir) -> Self {
        Self {
            temp_base: instance.path().to_path_buf(),
            instance: Mutex::new(Some(instance)),
            current: Mutex::new(None),
            latest: AtomicU64::new(0),
        }
    }

    pub fn temp_base(&self) -> &Path {
        &self.temp_base
    }

    /// Starts an open request. The ticket is also the new book's id.
    pub fn next_ticket(&self) -> u64 {
        self.latest.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn is_latest(&self, ticket: u64) -> bool {
        self.latest.load(Ordering::SeqCst) == ticket
    }

    /// Makes `book` current if no newer request started. Otherwise drops it,
    /// which also deletes its temporary folder.
    pub fn install(&self, book: Book) -> bool {
        let mut current = self.current.lock().unwrap();
        if !self.is_latest(book.id) {
            return false;
        }
        *current = Some(Arc::new(book));
        true
    }

    pub fn current(&self) -> Option<Arc<Book>> {
        self.current.lock().unwrap().clone()
    }

    pub fn shutdown(&self) {
        self.current.lock().unwrap().take();
        if let Some(instance) = self.instance.lock().unwrap().take() {
            instance.remove();
        }
    }
}
```

- [ ] **Step 4: Implement `src-tauri/src/protocol.rs`**

```rust
//! The comic:// scheme: `/{book_id}/{index}` returns one page of the current book.
//! `convertFileSrc` encodes `/` as `%2F`, so both forms are accepted.

use crate::book::Book;
use crate::image_entry::content_type;
use tauri::http::{header, Response};

pub fn parse_page_path(path: &str) -> Option<(u64, usize)> {
    let decoded = path.trim_start_matches('/').replace("%2F", "/").replace("%2f", "/");
    let (id, index) = decoded.split_once('/')?;
    Some((id.parse().ok()?, index.parse().ok()?))
}

fn status(code: u16) -> Response<Vec<u8>> {
    Response::builder().status(code).body(Vec::new()).unwrap()
}

pub fn page_response(book: Option<&Book>, path: &str) -> Response<Vec<u8>> {
    let Some((id, index)) = parse_page_path(path) else { return status(400) };
    let Some(book) = book.filter(|b| b.id == id) else { return status(404) };
    let Some(name) = book.page_name(index) else { return status(404) };
    match book.read_page(index) {
        Ok(bytes) => Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, content_type(name))
            .body(bytes)
            .unwrap(),
        Err(_) => status(500),
    }
}
```

- [ ] **Step 5: Implement `src-tauri/src/launch_open.rs`**

```rust
//! Paths that macOS asks the app to open (Dock drop, Finder "Open With").
//! The UI may not be ready yet, so the path waits here until the UI takes it.

use std::sync::Mutex;

pub const OPEN_EVENT: &str = "open-path";

#[derive(Default)]
pub struct PendingOpen(Mutex<Option<String>>);

impl PendingOpen {
    pub fn set(&self, path: String) {
        *self.0.lock().unwrap() = Some(path);
    }

    pub fn take(&self) -> Option<String> {
        self.0.lock().unwrap().take()
    }
}
```

- [ ] **Step 6: Implement `src-tauri/src/commands.rs`**

```rust
use crate::book::{Book, BookInfo};
use crate::launch_open::PendingOpen;
use crate::state::AppState;
use std::path::{Path, PathBuf};
use tauri::State;

pub const SUPERSEDED: &str = "superseded";

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn open_book(path: String, state: State<'_, AppState>) -> Result<BookInfo, String> {
    let ticket = state.next_ticket();
    let path = PathBuf::from(path);
    let base = state.temp_base().to_path_buf();
    let source_path = path.clone();
    let result = tauri::async_runtime::spawn_blocking(move || Book::open(&source_path, ticket, &base))
        .await
        .map_err(|e| format!("Internal error: {e}"))?;
    if !state.is_latest(ticket) {
        return Err(SUPERSEDED.into());
    }
    let book = result.map_err(|e| e.message(&display_name(&path)))?;
    let info = book.info();
    if state.install(book) {
        Ok(info)
    } else {
        Err(SUPERSEDED.into())
    }
}

#[tauri::command]
pub fn take_pending_open(pending: State<'_, PendingOpen>) -> Option<String> {
    pending.take()
}
```

- [ ] **Step 7: Replace `src-tauri/src/lib.rs`**

```rust
pub mod book;
mod commands;
pub mod image_entry;
pub mod launch_open;
pub mod natural_sort;
pub mod protocol;
pub mod source;
pub mod state;
pub mod temp;

use launch_open::PendingOpen;
use state::AppState;
#[cfg(target_os = "macos")]
use tauri::Emitter;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let base = temp::default_base();
    temp::cleanup_stale(&base);
    let instance = temp::InstanceDir::create(&base).expect("could not create temporary folder");

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new(instance))
        .manage(PendingOpen::default())
        .register_asynchronous_uri_scheme_protocol("comic", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let path = request.uri().path().to_owned();
            tauri::async_runtime::spawn_blocking(move || {
                let book = app.state::<AppState>().current();
                responder.respond(protocol::page_response(book.as_deref(), &path));
            });
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_book,
            commands::take_pending_open
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| match event {
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Opened { urls } => {
            if let Some(path) = urls.iter().find_map(|url| url.to_file_path().ok()) {
                app.state::<PendingOpen>().set(path.to_string_lossy().into_owned());
                let _ = app.emit(launch_open::OPEN_EVENT, ());
            }
        }
        tauri::RunEvent::Exit => app.state::<AppState>().shutdown(),
        _ => {}
    });
}
```

- [ ] **Step 8: Run all Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all unit and integration tests PASS (natural_sort 9, image_entry 11, source 3, temp 4, sources 19, book 9, app 6). No warnings about unused imports on macOS.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src src-tauri/tests/app.rs
git commit -m "feat: serve pages over comic:// and add open_book command

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 11: Frontend pure logic

**Files:**
- Create: `src/types.ts`, `src/zoom.ts`, `src/zoom.test.ts`, `src/layout.ts`, `src/layout.test.ts`, `src/keys.ts`, `src/keys.test.ts`, `src/text.ts`, `src/text.test.ts`

**Interfaces:**
- Produces:
  - `types.ts`: `interface PageInfo { name: string; width: number | null; height: number | null }`, `interface BookInfo { bookId: number; title: string; pages: PageInfo[] }`, `const SUPERSEDED = "superseded"`.
  - `zoom.ts`: `ZOOM_MIN = 25`, `ZOOM_MAX = 200`, `ZOOM_STEP = 10`, `ZOOM_DEFAULT = 100`, `clampZoom(z)`, `zoomIn(z)`, `zoomOut(z)`, `loadZoom(storage: Pick<Storage, "getItem"> | null): number`, `saveZoom(storage: Pick<Storage, "setItem"> | null, zoom: number): void`.
  - `layout.ts`: `interface PageBox { top: number; height: number }`, `interface Anchor { index: number; fraction: number }`, `currentPageIndex(boxes, y): number` (−1 when empty), `captureAnchor(boxes, scrollTop, viewportHeight): Anchor | null`, `restoreScrollTop(anchor, boxes, viewportHeight): number`.
  - `keys.ts`: `type KeyAction = "zoom-in" | "zoom-out" | "zoom-reset" | "fullscreen" | "exit-fullscreen"`, `keyAction(e: { key: string; ctrlKey: boolean; metaKey: boolean; isMac: boolean }): KeyAction | null`.
  - `text.ts`: `displayName(path)`, `openingLabel(path)`, `formatCounter(current, total)`.

- [ ] **Step 1: Write the failing tests**

`src/zoom.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { clampZoom, loadZoom, saveZoom, zoomIn, zoomOut, ZOOM_DEFAULT } from "./zoom";

describe("zoom", () => {
  it("clamps to 25..200 and rounds", () => {
    expect(clampZoom(10)).toBe(25);
    expect(clampZoom(250)).toBe(200);
    expect(clampZoom(99.6)).toBe(100);
    expect(clampZoom(Number.NaN)).toBe(ZOOM_DEFAULT);
  });

  it("steps by 10 within limits", () => {
    expect(zoomIn(100)).toBe(110);
    expect(zoomOut(100)).toBe(90);
    expect(zoomIn(195)).toBe(200);
    expect(zoomIn(200)).toBe(200);
    expect(zoomOut(30)).toBe(25);
    expect(zoomOut(25)).toBe(25);
  });

  it("loads saved zoom or falls back to default", () => {
    const store = new Map<string, string>();
    const storage = {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
    };
    expect(loadZoom(storage)).toBe(ZOOM_DEFAULT);
    saveZoom(storage, 140);
    expect(loadZoom(storage)).toBe(140);
    store.set("comic-reader.zoom", "garbage");
    expect(loadZoom(storage)).toBe(ZOOM_DEFAULT);
    store.set("comic-reader.zoom", "");
    expect(loadZoom(storage)).toBe(ZOOM_DEFAULT);
    expect(loadZoom(null)).toBe(ZOOM_DEFAULT);
  });

  it("survives storage that throws", () => {
    const broken = {
      getItem: () => { throw new Error("blocked"); },
      setItem: () => { throw new Error("blocked"); },
    };
    expect(loadZoom(broken)).toBe(ZOOM_DEFAULT);
    expect(() => saveZoom(broken, 120)).not.toThrow();
  });
});
```

`src/layout.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { captureAnchor, currentPageIndex, restoreScrollTop, type PageBox } from "./layout";

const boxes: PageBox[] = [
  { top: 0, height: 1000 },
  { top: 1000, height: 500 },
  { top: 1500, height: 1500 },
];

describe("currentPageIndex", () => {
  it("returns the page that contains y", () => {
    expect(currentPageIndex(boxes, 0)).toBe(0);
    expect(currentPageIndex(boxes, 999)).toBe(0);
    expect(currentPageIndex(boxes, 1000)).toBe(1);
    expect(currentPageIndex(boxes, 2999)).toBe(2);
    expect(currentPageIndex(boxes, 99999)).toBe(2);
  });

  it("handles empty and negative input", () => {
    expect(currentPageIndex([], 10)).toBe(-1);
    expect(currentPageIndex(boxes, -50)).toBe(0);
  });
});

describe("zoom anchor", () => {
  it("keeps the same point of the current page at the viewport center", () => {
    // Viewport 800 high, scrolled so the center (y=1250) is half-way into page 1.
    const anchor = captureAnchor(boxes, 850, 800);
    expect(anchor).toEqual({ index: 1, fraction: 0.5 });
    // After zoom to 200%, every page is twice as tall.
    const zoomed = boxes.map((b) => ({ top: b.top * 2, height: b.height * 2 }));
    expect(restoreScrollTop(anchor!, zoomed, 800)).toBe(2000 + 500 - 400);
  });

  it("never returns a negative scroll position", () => {
    const anchor = captureAnchor(boxes, 0, 800)!;
    expect(restoreScrollTop(anchor, boxes, 800)).toBe(0);
  });

  it("returns null without pages", () => {
    expect(captureAnchor([], 0, 800)).toBeNull();
  });
});
```

`src/keys.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { keyAction } from "./keys";

const mac = (key: string, mods: { ctrl?: boolean; meta?: boolean } = {}) =>
  keyAction({ key, ctrlKey: !!mods.ctrl, metaKey: !!mods.meta, isMac: true });
const win = (key: string, mods: { ctrl?: boolean; meta?: boolean } = {}) =>
  keyAction({ key, ctrlKey: !!mods.ctrl, metaKey: !!mods.meta, isMac: false });

describe("keyAction", () => {
  it("maps zoom shortcuts to Cmd on macOS and Ctrl on Windows", () => {
    expect(mac("=", { meta: true })).toBe("zoom-in");
    expect(mac("+", { meta: true })).toBe("zoom-in");
    expect(mac("-", { meta: true })).toBe("zoom-out");
    expect(mac("0", { meta: true })).toBe("zoom-reset");
    expect(mac("=", { ctrl: true })).toBeNull();
    expect(win("=", { ctrl: true })).toBe("zoom-in");
    expect(win("-", { ctrl: true })).toBe("zoom-out");
    expect(win("0", { ctrl: true })).toBe("zoom-reset");
    expect(win("=", { meta: true })).toBeNull();
  });

  it("maps full screen keys per platform", () => {
    expect(mac("f", { ctrl: true, meta: true })).toBe("fullscreen");
    expect(mac("F", { ctrl: true, meta: true })).toBe("fullscreen");
    expect(mac("F11")).toBeNull();
    expect(win("F11")).toBe("fullscreen");
    expect(mac("Escape")).toBe("exit-fullscreen");
    expect(win("Escape")).toBe("exit-fullscreen");
  });

  it("ignores other keys", () => {
    expect(win("a")).toBeNull();
    expect(mac("0")).toBeNull();
  });
});
```

`src/text.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { displayName, formatCounter, openingLabel } from "./text";

describe("text", () => {
  it("takes the last path segment on both platforms", () => {
    expect(displayName("/Users/me/Comics/Vol 1.cbz")).toBe("Vol 1.cbz");
    expect(displayName("C:\\Comics\\Vol 2.cbr")).toBe("Vol 2.cbr");
    expect(displayName("/Users/me/Comics/Folder/")).toBe("Folder");
  });

  it("says Extracting for rar and tar.gz, Opening otherwise", () => {
    expect(openingLabel("/a/b.CBR")).toBe("Extracting b.CBR…");
    expect(openingLabel("/a/b.rar")).toBe("Extracting b.rar…");
    expect(openingLabel("/a/b.tar.gz")).toBe("Extracting b.tar.gz…");
    expect(openingLabel("/a/b.tgz")).toBe("Extracting b.tgz…");
    expect(openingLabel("/a/b.cbz")).toBe("Opening b.cbz…");
    expect(openingLabel("/a/Folder")).toBe("Opening Folder…");
  });

  it("formats the page counter", () => {
    expect(formatCounter(12, 48)).toBe("12 / 48");
    expect(formatCounter(0, 0)).toBe("");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm test`
Expected: FAIL — modules `./zoom`, `./layout`, `./keys`, `./text` not found.

- [ ] **Step 3: Implement the modules**

`src/types.ts`:

```ts
export interface PageInfo {
  name: string;
  width: number | null;
  height: number | null;
}

export interface BookInfo {
  bookId: number;
  title: string;
  pages: PageInfo[];
}

/** Error string from open_book when a newer open replaced the request. */
export const SUPERSEDED = "superseded";
```

`src/zoom.ts`:

```ts
export const ZOOM_MIN = 25;
export const ZOOM_MAX = 200;
export const ZOOM_STEP = 10;
export const ZOOM_DEFAULT = 100;

const STORAGE_KEY = "comic-reader.zoom";

export function clampZoom(zoom: number): number {
  if (!Number.isFinite(zoom)) return ZOOM_DEFAULT;
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Math.round(zoom)));
}

export const zoomIn = (zoom: number): number => clampZoom(zoom + ZOOM_STEP);
export const zoomOut = (zoom: number): number => clampZoom(zoom - ZOOM_STEP);

export function loadZoom(storage: Pick<Storage, "getItem"> | null): number {
  try {
    const raw = storage?.getItem(STORAGE_KEY);
    if (raw == null || raw === "") return ZOOM_DEFAULT;
    return clampZoom(Number(raw));
  } catch {
    return ZOOM_DEFAULT;
  }
}

export function saveZoom(storage: Pick<Storage, "setItem"> | null, zoom: number): void {
  try {
    storage?.setItem(STORAGE_KEY, String(zoom));
  } catch {
    // Storage blocked: the zoom is simply not remembered.
  }
}
```

`src/layout.ts`:

```ts
export interface PageBox {
  top: number;
  height: number;
}

/** A point inside a page: which page, and how far down it (0..1). */
export interface Anchor {
  index: number;
  fraction: number;
}

/** Index of the last page whose top is at or above `y`; -1 when there are no pages. */
export function currentPageIndex(boxes: PageBox[], y: number): number {
  if (boxes.length === 0) return -1;
  let lo = 0;
  let hi = boxes.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (boxes[mid].top <= y) lo = mid;
    else hi = mid - 1;
  }
  return lo;
}

export function captureAnchor(boxes: PageBox[], scrollTop: number, viewportHeight: number): Anchor | null {
  const center = scrollTop + viewportHeight / 2;
  const index = currentPageIndex(boxes, center);
  if (index < 0) return null;
  const box = boxes[index];
  const fraction = box.height > 0 ? Math.min(Math.max((center - box.top) / box.height, 0), 1) : 0;
  return { index, fraction };
}

export function restoreScrollTop(anchor: Anchor, boxes: PageBox[], viewportHeight: number): number {
  const box = boxes[anchor.index];
  if (!box) return 0;
  return Math.max(0, box.top + anchor.fraction * box.height - viewportHeight / 2);
}
```

`src/keys.ts`:

```ts
export type KeyAction = "zoom-in" | "zoom-out" | "zoom-reset" | "fullscreen" | "exit-fullscreen";

export interface KeyInput {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  isMac: boolean;
}

export function keyAction({ key, ctrlKey, metaKey, isMac }: KeyInput): KeyAction | null {
  const mod = isMac ? metaKey && !ctrlKey : ctrlKey && !metaKey;
  if (mod && (key === "+" || key === "=")) return "zoom-in";
  if (mod && key === "-") return "zoom-out";
  if (mod && key === "0") return "zoom-reset";
  if (!isMac && key === "F11") return "fullscreen";
  if (isMac && ctrlKey && metaKey && key.toLowerCase() === "f") return "fullscreen";
  if (key === "Escape") return "exit-fullscreen";
  return null;
}
```

`src/text.ts`:

```ts
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test`
Expected: 4 test files, 15 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/types.ts src/zoom.ts src/zoom.test.ts src/layout.ts src/layout.test.ts src/keys.ts src/keys.test.ts src/text.ts src/text.test.ts
git commit -m "feat: add frontend zoom, layout, key, and text logic

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 12: Viewer UI, toolbar, full screen, and open flow

**Files:**
- Create: `src/viewer.ts`, `src/toolbar.ts`, `src/fullscreen.ts`
- Modify: `src/main.ts` (full replacement)

**Interfaces:**
- Consumes: everything in Task 11; Tauri commands `open_book`, `take_pending_open`; event `open-path` (Task 10); element ids from Task 1.
- Produces: `class Viewer { constructor(scroller: HTMLElement, column: HTMLElement); show(book: BookInfo): void; clear(): void; setZoom(zoom: number): void; onPageChange: (current: number, total: number) => void }`, `setupToolbar(handlers: ToolbarHandlers)`, `setZoomLabel(zoom)`, `setCounter(text)`, `class Fullscreen { toggle(): Promise<void>; exit(): Promise<void> }`, and `openPath(path: string)` inside `main.ts` (used by Task 13 via the `onDrop` callback).

These modules touch the DOM and Tauri APIs, so they are checked by hand in Step 6 rather than by Vitest. All math they use is already tested in Task 11.

- [ ] **Step 1: Implement `src/viewer.ts`**

```ts
import { convertFileSrc } from "@tauri-apps/api/core";
import { captureAnchor, currentPageIndex, restoreScrollTop, type PageBox } from "./layout";
import type { BookInfo } from "./types";

const UNKNOWN_RATIO = "2 / 3";
/** Pages within about two screen heights of the viewport stay loaded. */
const LOAD_MARGIN = "200% 0px";

export class Viewer {
  onPageChange: (current: number, total: number) => void = () => {};

  private readonly scroller: HTMLElement;
  private readonly column: HTMLElement;
  private readonly observer: IntersectionObserver;
  private pages: HTMLElement[] = [];
  private book: BookInfo | null = null;
  private frame = 0;

  constructor(scroller: HTMLElement, column: HTMLElement) {
    this.scroller = scroller;
    this.column = column;
    this.observer = new IntersectionObserver((entries) => this.onIntersect(entries), {
      root: scroller,
      rootMargin: LOAD_MARGIN,
    });
    scroller.addEventListener("scroll", () => this.scheduleUpdate(), { passive: true });
    new ResizeObserver(() => this.scheduleUpdate()).observe(scroller);
  }

  show(book: BookInfo): void {
    this.clear();
    this.book = book;
    const fragment = document.createDocumentFragment();
    book.pages.forEach((page, index) => {
      const el = document.createElement("div");
      el.className = "page";
      el.dataset.index = String(index);
      el.style.aspectRatio = page.width && page.height ? `${page.width} / ${page.height}` : UNKNOWN_RATIO;

      const img = document.createElement("img");
      img.alt = page.name;
      img.decoding = "async";
      img.draggable = false;
      img.addEventListener("load", () => {
        el.classList.remove("failed");
        if (!page.width || !page.height) {
          el.style.aspectRatio = `${img.naturalWidth} / ${img.naturalHeight}`;
        }
      });
      img.addEventListener("error", () => {
        if (!img.getAttribute("src")) return;
        el.classList.add("failed");
        el.dataset.message = `Page ${index + 1} could not be loaded (${page.name})`;
      });

      el.appendChild(img);
      fragment.appendChild(el);
      this.pages.push(el);
    });
    this.column.appendChild(fragment);
    this.pages.forEach((el) => this.observer.observe(el));
    this.scroller.scrollTo(0, 0);
    this.scheduleUpdate();
  }

  clear(): void {
    this.observer.disconnect();
    this.pages.forEach((el) => el.querySelector("img")?.removeAttribute("src"));
    this.column.replaceChildren();
    this.pages = [];
    this.book = null;
  }

  setZoom(zoom: number): void {
    const anchor = captureAnchor(this.boxes(), this.scroller.scrollTop, this.scroller.clientHeight);
    document.documentElement.style.setProperty("--zoom", String(zoom));
    if (anchor) {
      this.scroller.scrollTop = restoreScrollTop(anchor, this.boxes(), this.scroller.clientHeight);
    }
    this.scheduleUpdate();
  }

  private boxes(): PageBox[] {
    return this.pages.map((el) => ({ top: el.offsetTop, height: el.offsetHeight }));
  }

  private onIntersect(entries: IntersectionObserverEntry[]): void {
    const book = this.book;
    if (!book) return;
    for (const entry of entries) {
      const el = entry.target as HTMLElement;
      const img = el.querySelector("img");
      if (!img) continue;
      if (entry.isIntersecting && !img.getAttribute("src")) {
        img.src = convertFileSrc(`${book.bookId}/${el.dataset.index}`, "comic");
      } else if (!entry.isIntersecting && img.getAttribute("src")) {
        img.removeAttribute("src"); // frees the decoded image
      }
    }
  }

  private scheduleUpdate(): void {
    if (this.frame) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      const total = this.pages.length;
      const center = this.scroller.scrollTop + this.scroller.clientHeight / 2;
      const index = currentPageIndex(this.boxes(), center);
      this.onPageChange(index + 1, total);
    });
  }
}
```

`#scroller` has `position: relative` (Task 1 CSS), so each page's `offsetTop` is measured from the top of the scroll content.

- [ ] **Step 2: Implement `src/toolbar.ts`**

```ts
export interface ToolbarHandlers {
  openFile(): void;
  openFolder(): void;
  zoomIn(): void;
  zoomOut(): void;
  toggleFullscreen(): void;
}

function byId(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`Missing #${id}`);
  return el;
}

function bind(selector: string, handler: () => void): void {
  document.querySelectorAll<HTMLElement>(selector).forEach((el) => el.addEventListener("click", handler));
}

export function setupToolbar(h: ToolbarHandlers): void {
  bind("#open-file, [data-open=file]", h.openFile);
  bind("#open-folder, [data-open=folder]", h.openFolder);
  bind("#zoom-in", h.zoomIn);
  bind("#zoom-out", h.zoomOut);
  bind("#fullscreen", h.toggleFullscreen);
}

export function setZoomLabel(zoom: number): void {
  byId("zoom-label").textContent = `${zoom}%`;
}

export function setCounter(text: string): void {
  byId("counter").textContent = text;
  byId("counter-overlay").textContent = text;
}
```

- [ ] **Step 3: Implement `src/fullscreen.ts`**

```ts
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Mouse within this distance of the top edge shows the toolbar in full screen. */
const REVEAL_PX = 40;
/** Mouse below this distance hides it again (the gap avoids flicker). */
const HIDE_PX = 80;

export class Fullscreen {
  private active = false;

  constructor() {
    // Also catches full screen changes from the macOS green button and menu.
    void getCurrentWindow().onResized(() => void this.sync());
    document.addEventListener("mousemove", (e) => {
      if (!this.active) return;
      if (e.clientY <= REVEAL_PX) document.body.classList.add("reveal-toolbar");
      else if (e.clientY > HIDE_PX) document.body.classList.remove("reveal-toolbar");
    });
    void this.sync();
  }

  async toggle(): Promise<void> {
    await getCurrentWindow().setFullscreen(!this.active);
    await this.sync();
  }

  async exit(): Promise<void> {
    if (!this.active) return;
    await getCurrentWindow().setFullscreen(false);
    await this.sync();
  }

  private async sync(): Promise<void> {
    this.active = await getCurrentWindow().isFullscreen();
    document.body.classList.toggle("fullscreen", this.active);
    if (!this.active) document.body.classList.remove("reveal-toolbar");
  }
}
```

- [ ] **Step 4: Replace `src/main.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { Fullscreen } from "./fullscreen";
import { keyAction } from "./keys";
import { formatCounter, openingLabel } from "./text";
import { setCounter, setupToolbar, setZoomLabel } from "./toolbar";
import { SUPERSEDED, type BookInfo } from "./types";
import { Viewer } from "./viewer";
import { loadZoom, saveZoom, zoomIn, zoomOut, ZOOM_DEFAULT } from "./zoom";

const ARCHIVE_EXTENSIONS = ["zip", "cbz", "rar", "cbr", "gz", "tgz"];
const ERROR_VISIBLE_MS = 6000;
const isMac = navigator.userAgent.includes("Mac");

function byId(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`Missing #${id}`);
  return el;
}

function safeLocalStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

const storage = safeLocalStorage();
const viewer = new Viewer(byId("scroller"), byId("pages"));
const fullscreen = new Fullscreen();
const status = byId("status");
let zoom = loadZoom(storage);
let statusTimer = 0;

function showStatus(text: string, isError = false): void {
  window.clearTimeout(statusTimer);
  status.textContent = text;
  status.classList.toggle("error", isError);
  status.hidden = false;
  if (isError) statusTimer = window.setTimeout(hideStatus, ERROR_VISIBLE_MS);
}

function hideStatus(): void {
  window.clearTimeout(statusTimer);
  status.hidden = true;
}

status.addEventListener("click", () => {
  if (status.classList.contains("error")) hideStatus();
});

export async function openPath(path: string): Promise<void> {
  showStatus(openingLabel(path));
  try {
    const book = await invoke<BookInfo>("open_book", { path });
    hideStatus();
    viewer.show(book);
    document.body.classList.add("has-book");
    await getCurrentWindow().setTitle(`${book.title} — Comic Reader`);
  } catch (err) {
    if (err === SUPERSEDED) return; // a newer open owns the status line
    showStatus(String(err), true);
  }
}

async function chooseFile(): Promise<void> {
  const path = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Comics", extensions: ARCHIVE_EXTENSIONS }],
  });
  if (typeof path === "string") await openPath(path);
}

async function chooseFolder(): Promise<void> {
  const path = await open({ multiple: false, directory: true });
  if (typeof path === "string") await openPath(path);
}

function applyZoom(next: number): void {
  zoom = next;
  viewer.setZoom(zoom);
  setZoomLabel(zoom);
  saveZoom(storage, zoom);
}

setupToolbar({
  openFile: () => void chooseFile(),
  openFolder: () => void chooseFolder(),
  zoomIn: () => applyZoom(zoomIn(zoom)),
  zoomOut: () => applyZoom(zoomOut(zoom)),
  toggleFullscreen: () => void fullscreen.toggle(),
});

viewer.onPageChange = (current, total) => setCounter(formatCounter(current, total));

document.addEventListener("keydown", (e) => {
  const action = keyAction({ key: e.key, ctrlKey: e.ctrlKey, metaKey: e.metaKey, isMac });
  if (!action) return;
  e.preventDefault();
  switch (action) {
    case "zoom-in":
      applyZoom(zoomIn(zoom));
      break;
    case "zoom-out":
      applyZoom(zoomOut(zoom));
      break;
    case "zoom-reset":
      applyZoom(ZOOM_DEFAULT);
      break;
    case "fullscreen":
      void fullscreen.toggle();
      break;
    case "exit-fullscreen":
      void fullscreen.exit();
      break;
  }
});

applyZoom(zoom);

async function openPending(): Promise<void> {
  const path = await invoke<string | null>("take_pending_open");
  if (path) await openPath(path);
}

// A Dock drop can arrive before this script runs (stored by the backend) or
// later (signalled by the event). Listen first, then check the store.
void listen("open-path", () => void openPending()).then(openPending);
```

- [ ] **Step 5: Check types and tests**

Run: `npm run build && npm test`
Expected: `tsc` reports no errors; Vite writes `dist/`; 15 tests PASS.

- [ ] **Step 6: Check by hand**

Run: `npm run tauri dev`, then:
1. Click "Open Folder" and choose a folder with `page 1.jpg` … `page 12.jpg`. Expected: pages show in order 1, 2, … 12, each as wide as the window, with no gap. The window title shows the folder name.
2. Scroll. Expected: the counter changes from `1 / 12` to `12 / 12`.
3. Press `Cmd/Ctrl +` three times. Expected: label `130%`, a horizontal scroll bar shows, and the same page stays at the screen center. Press `Cmd/Ctrl 0`: back to `100%`.
4. Quit and start the app again. Expected: the zoom label shows the last zoom.
5. Click "Full Screen". Expected: the toolbar hides and the counter shows in the bottom-right corner. Move the mouse to the top edge: the toolbar shows. Press `Esc`: full screen ends.
6. Click "Open File" and choose `src-tauri/tests/fixtures/book.cbr`. Expected: "Extracting book.cbr…" shows briefly, then 3 colored pages in order green, red, blue.
7. Open `src-tauri/tests/fixtures/encrypted-files.rar`. Expected: the red message "encrypted-files.rar is password protected. Password-protected archives are not supported." The previous book stays open.

- [ ] **Step 7: Commit**

```bash
git add src/viewer.ts src/toolbar.ts src/fullscreen.ts src/main.ts
git commit -m "feat: add vertical page viewer with zoom, full screen, and counter

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 13: Drag and drop, and macOS Dock open

**Files:**
- Create: `src/dragdrop.ts`, `src-tauri/tauri.macos.conf.json`
- Modify: `src/main.ts`

**Interfaces:**
- Consumes: `openPath` (Task 12); `RunEvent::Opened` handling and `take_pending_open` (Task 10).
- Produces: `setupDragDrop(onDrop: (path: string) => void): Promise<void>`; macOS document types for `zip cbz rar cbr gz tgz` and `public.folder`.

- [ ] **Step 1: Implement `src/dragdrop.ts`**

```ts
import { getCurrentWebview } from "@tauri-apps/api/webview";

export async function setupDragDrop(onDrop: (path: string) => void): Promise<void> {
  const highlight = document.getElementById("drop-highlight");
  if (!highlight) throw new Error("Missing #drop-highlight");
  await getCurrentWebview().onDragDropEvent((event) => {
    const payload = event.payload;
    if (payload.type === "enter" || payload.type === "over") {
      highlight.hidden = false;
      return;
    }
    highlight.hidden = true;
    if (payload.type === "drop" && payload.paths.length > 0) onDrop(payload.paths[0]);
  });
}
```

- [ ] **Step 2: Wire it into `src/main.ts`**

Add the import:

```ts
import { setupDragDrop } from "./dragdrop";
```

Add after `applyZoom(zoom);`:

```ts
void setupDragDrop((path) => void openPath(path));
```

- [ ] **Step 3: Create `src-tauri/tauri.macos.conf.json`**

```json
{
  "bundle": {
    "fileAssociations": [
      { "ext": ["cbz", "zip"], "name": "Zip Comic", "role": "Viewer", "rank": "Alternate" },
      { "ext": ["cbr", "rar"], "name": "RAR Comic", "role": "Viewer", "rank": "Alternate" },
      { "ext": ["tgz", "gz"], "name": "Gzip Tarball", "role": "Viewer", "rank": "Alternate" },
      { "ext": [], "contentTypes": ["public.folder"], "name": "Folder", "role": "Viewer", "rank": "Alternate" }
    ]
  }
}
```

`CFBundleTypeExtensions` matches only the last extension, so `.tar.gz` is declared as `gz`. Tauri writes `contentTypes` to `LSItemContentTypes` and omits `CFBundleTypeExtensions` when `ext` is empty.

- [ ] **Step 4: Check types**

Run: `npm run build`
Expected: no `tsc` errors.

- [ ] **Step 5: Check window drop by hand**

Run: `npm run tauri dev`. Drag `src-tauri/tests/fixtures/book.rar` onto the window.
Expected: a dashed blue frame with "Drop to open" shows while dragging; after the drop, the 3 pages show. Drag a folder of images onto the window: the folder opens. Drag a file out of the window and back out again without dropping: the frame hides.

- [ ] **Step 6: Build the macOS app bundle and check `Info.plist`**

Run:
```bash
npm run tauri build -- --bundles app
plutil -p "src-tauri/target/release/bundle/macos/Comic Reader.app/Contents/Info.plist" | grep -A 40 CFBundleDocumentTypes
```
Expected: four document types. The "Folder" type has `LSItemContentTypes => ["public.folder"]`; every type has `CFBundleTypeRole => "Viewer"` and `LSHandlerRank => "Alternate"`.

If the folder entry is missing, create `src-tauri/Info.plist` with this content (Tauri merges it into the generated `Info.plist`), and rebuild:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>Folder</string>
      <key>CFBundleTypeRole</key><string>Viewer</string>
      <key>LSHandlerRank</key><string>Alternate</string>
      <key>LSItemContentTypes</key><array><string>public.folder</string></array>
    </dict>
  </array>
</dict>
</plist>
```

- [ ] **Step 7: Check Dock open by hand (macOS)**

1. Copy `Comic Reader.app` to `/Applications` and start it. Keep it in the Dock (right-click the Dock icon > Options > Keep in Dock).
2. With the app running, drag `book.cbr` onto the Dock icon. Expected: the book opens.
3. Quit the app. Drag a folder of images onto the Dock icon. Expected: the app starts and opens the folder.
4. In Finder, right-click a `.cbz` file > Open With. Expected: "Comic Reader" is in the list, and the default app for `.cbz` has not changed.

- [ ] **Step 8: Commit**

```bash
git add src/dragdrop.ts src/main.ts src-tauri/tauri.macos.conf.json
git commit -m "feat: open books by window drop and macOS Dock drop

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

If Step 6 needed `src-tauri/Info.plist`, add it to the same commit.

---

### Task 14: Release builds and manual checklist

**Files:**
- Create: `README.md`

**Interfaces:**
- Consumes: the complete app.
- Produces: `.dmg` on macOS, `.msi` and NSIS `.exe` on Windows; a README with build and test commands.

- [ ] **Step 1: Create `README.md`**

````markdown
# Comic Reader

A desktop reader for comics and image sets on macOS and Windows. It opens a
folder, `.zip`/`.cbz`, `.rar`/`.cbr`, or `.tar.gz`/`.tgz`, sorts the pages in
natural order (`page 2` before `page 10`), and shows them in one vertical
column that fits the window width.

## Requirements

- Rust stable 1.89 or later
- Node.js LTS
- macOS: Xcode Command Line Tools
- Windows: Microsoft C++ Build Tools and WebView2 (included in Windows 11)

## Development

```bash
npm install
npm run tauri dev
```

## Tests

```bash
npm run build                                        # once: the Rust build embeds dist/
cargo test --manifest-path src-tauri/Cargo.toml
npm test
```

RAR fixtures are committed. To recreate them, install the rar CLI
(`brew install --cask rar`) and run `scripts/make-rar-fixtures.sh`.

## Release build

Build on each platform:

```bash
npm run tauri build
```

- macOS: `src-tauri/target/release/bundle/dmg/`
- Windows: `src-tauri/target/release/bundle/msi/` and `bundle/nsis/`

The app is not signed. On macOS, right-click the app and choose Open the first time.
````

- [ ] **Step 2: Run all automated tests**

Run: `npm run build && npm test && cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all tests PASS.

- [ ] **Step 3: Build on macOS**

Run: `npm run tauri build`
Expected: a `.dmg` in `src-tauri/target/release/bundle/dmg/`.

- [ ] **Step 4: Run the macOS manual checklist**

- [ ] Open by "Open File", "Open Folder", window drop, and Dock drop (app closed and app running).
- [ ] A book with 500+ pages: scroll fast from top to bottom and back. In Activity Monitor, the "Comic Reader Web Content" memory goes up and comes back down; it does not grow without limit.
- [ ] Full screen: button, `Ctrl+Cmd+F`, green window button, `Esc`. Toolbar reveal at the top edge.
- [ ] Zoom: 25% and 200% limits; position kept; value remembered after restart.
- [ ] Page counter updates and is correct at the first and last page.
- [ ] Each error in spec section 7: damaged archive, password-protected archive, a folder with no images, a page file deleted while the book is open (that page shows "Page N could not be loaded").
- [ ] Open a second book while a large RAR is still extracting: only the second book shows.
- [ ] Quit the app, then check that `$TMPDIR/comic-reader/` has no `inst-*` folder left.

- [ ] **Step 5: Build and check on Windows**

On a Windows machine or VM with the requirements from the README:

```bash
git clone <repo> && cd <repo>
npm install
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm test
npm run tauri build
```

Install the `.msi`, then run the same checklist as Step 4 with these changes:
- [ ] No Dock or "Open With" checks. Window drop works.
- [ ] Full screen uses `F11`.
- [ ] Zoom uses `Ctrl +`, `Ctrl −`, `Ctrl 0`, and the WebView's own zoom does not change.
- [ ] Pages load (this confirms `convertFileSrc` produces working `http://comic.localhost/...` URLs).
- [ ] After quitting, `%TEMP%\comic-reader\` has no `inst-*` folder left.

- [ ] **Step 6: Commit**

```bash
git add README.md
git commit -m "docs: add README with build, test, and release steps

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```
