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

RAR fixtures are committed. To recreate them, install the rar CLI from
https://www.rarlab.com/download.htm, put `rar` on PATH, and run
`scripts/make-rar-fixtures.sh`.

## Release build

Build on each platform:

```bash
npm run tauri build
```

- macOS: `src-tauri/target/release/bundle/dmg/`
- Windows: `src-tauri/target/release/bundle/msi/` and `bundle/nsis/`

The app is not signed. On macOS, right-click the app and choose Open the first time.
