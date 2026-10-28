# Contributing

Thanks for helping improve nvim-md-preview. Bug reports, ideas and pull requests are all welcome.

## Project layout

| Path | What it is |
|---|---|
| `plugin/md_preview.lua` | User commands |
| `lua/md_preview.lua` | Neovim side: starts the binary, sends the buffer and cursor, install, export |
| `lua/md_preview/health.lua` | `:checkhealth md_preview` |
| `src/main.rs` | msgpack-rpc loop on stdin/stdout and the handlers for messages from Neovim |
| `src/markdown.rs` | Markdown → HTML (pulldown-cmark), source-line markers, wiki links, math |
| `src/server.rs` | Starts the HTTP / WebSocket server, routes, and the WebSocket to the browser |
| `src/state.rs` | Shared state and settings received from Neovim |
| `src/access.rs` | Access control for every request (Host / Origin checks, LAN token) |
| `src/assets.rs` | Embedded JS / CSS / fonts, and images from the document's folder |
| `src/page.rs` | The preview page, the print page used for PDFs, and browser settings |
| `src/export.rs` | Export: Markdown, pandoc, Typst, and printing PDFs with a browser |
| `src/preview/` | The preview page itself: `index.html`, `style.css`, `app.js` |
| `download-libs.sh` | Fetches the JS/CSS libraries embedded at build time |
| `tests/smoke.lua` | Headless Neovim smoke test |
| `tests/e2e/` | Browser tests with Playwright |

## Development

```bash
bash download-libs.sh            # once; required by build.rs
cargo build --release
cargo test --release             # Rust unit tests
cargo clippy --release -- -D warnings
nvim --headless -u NONE -c "luafile tests/smoke.lua" -c "qa!"
```

To try your build, point the plugin at it:

```lua
require("md_preview").setup({ binary = vim.fn.getcwd() .. "/target/release/nvim-md-preview" })
```

The preview page lives in `src/preview/` and is embedded into the binary at build time, so rebuild the binary after changing HTML, CSS or JS.

### Browser tests

`tests/e2e` starts the preview in a headless Neovim for each test and checks it in Chromium with Playwright: rendering, typing, scroll sync, click-to-jump, the toolbar, slides, search, the Git diff, the Export menu and the server's access checks. They need Node.js and the release binary:

```bash
cargo build --release
cd tests/e2e
npm ci
npx playwright install chromium
npx playwright test
```

Add a test here when you fix a bug that can only be seen in the browser. Set `MD_PREVIEW_BINARY` to run the tests against another binary.

## Embedded libraries

`download-libs.sh` fetches Mermaid, KaTeX and highlight.js into `src/embedded_lib/` and checks each file against a pinned SHA-256 checksum. To update a library, change its `*_VERSION` in the script, run `bash download-libs.sh --update-checksums`, paste the printed lines into `CHECKSUMS`, and run the script again to confirm it passes. Check the upstream changelog before bumping a major version.

## Pull requests

- Keep each PR focused on one change and describe how you verified it.
- CI runs the build and Rust tests on Linux, macOS and Windows, clippy with `-D warnings`, the Lua smoke test on all three, and the browser tests. Please make sure they pass locally.
- Add a unit test in `src/markdown.rs` when you change Markdown rendering.
- Update `README.md`, `README.ja.md` and `doc/md_preview.txt` when you add or change options or commands.
- Add an entry under "Unreleased" in `CHANGELOG.md`.

## Security

The server must stay safe to run while the user browses other sites. Do not add CORS headers, do not serve files outside the document directory, and keep the Host / Origin checks in `access_guard`. Please report vulnerabilities privately through GitHub's "Report a vulnerability" on the Security tab rather than in a public issue.

## Releases

Releases are built by `.github/workflows/release.yml` when a `v*` tag is pushed. Use semantic versioning and:

1. Bump `version` in `Cargo.toml` and `M.version` in `lua/md_preview.lua` to the same value. CI fails if they differ, and the release fails if they do not match the tag.
2. Move the "Unreleased" changelog entries under the new version. That section becomes the release notes.
3. Merge, then push a new tag such as `v0.6.0`. Do not move an existing tag.

The release is created as a draft and published only after every platform has uploaded its files.

`:MdPreviewInstall` downloads the binary from the release that matches `M.version` (falling back to the latest release when that version has not been released yet), and the plugin warns when the installed binary's version differs.
