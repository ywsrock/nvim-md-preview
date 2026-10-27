# Contributing

Thanks for helping improve nvim-md-preview. Bug reports, ideas and pull requests are all welcome.

## Project layout

| Path | What it is |
|---|---|
| `plugin/md_preview.lua` | User commands |
| `lua/md_preview.lua` | Neovim side: starts the binary, sends the buffer and cursor, install, export |
| `lua/md_preview/health.lua` | `:checkhealth md_preview` |
| `src/main.rs` | msgpack-rpc loop on stdin/stdout |
| `src/markdown.rs` | Markdown → HTML (pulldown-cmark), source-line markers, wiki links, math |
| `src/server.rs` | HTTP / WebSocket server, access control, export, and the preview page (HTML/CSS/JS) |
| `download-libs.sh` | Fetches the JS/CSS libraries embedded at build time |
| `tests/smoke.lua` | Headless Neovim smoke test |

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

The preview page is embedded in `src/server.rs`, so rebuild the binary after changing HTML, CSS or JS.

## Pull requests

- Keep each PR focused on one change and describe how you verified it.
- CI runs the build, Rust tests, clippy with `-D warnings` and the Lua smoke test. Please make sure they pass locally.
- Add a unit test in `src/markdown.rs` when you change Markdown rendering.
- Update `README.md`, `README.ja.md` and `doc/md_preview.txt` when you add or change options or commands.
- Add an entry under "Unreleased" in `CHANGELOG.md`.

## Security

The server must stay safe to run while the user browses other sites. Do not add CORS headers, do not serve files outside the document directory, and keep the Host / Origin checks in `access_guard`. Please report vulnerabilities privately through GitHub's "Report a vulnerability" on the Security tab rather than in a public issue.

## Releases

Releases are built by `.github/workflows/release.yml` when a `v*` tag is pushed. Use semantic versioning, bump `version` in `Cargo.toml`, move the "Unreleased" changelog entries under the new version, and push a new tag. Do not move an existing tag.
