# Changelog

All notable changes are documented here. This project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed
- Running `:MdPreview` again after closing the browser tab did not open the browser (since 0.8.0). When the server is already running, the preview URL is now opened again right away.

## [0.8.0] - 2026-10-28

### Added
- PlantUML diagrams in ` ```plantuml ` and ` ```puml ` blocks. They are drawn with the local `plantuml` command (or a `plantuml.jar` set in `plantuml_path`). A PlantUML server is used only when `plantuml_server` is set, so diagram sources never leave the machine by default. Diagrams are included in browser-printed PDFs. `:checkhealth md_preview` shows which one is used.

### Fixed
- WSL: when a preview was already running in Neovim on Windows, starting one in WSL opened the Windows preview instead. WSL and Windows have separate `localhost` networks, so the WSL server could take the same port, but the browser (on Windows) reached the Windows server. In WSL the server now skips ports that are in use on Windows (checked with `netstat.exe`).
- The browser is opened after the server has picked its port, instead of after a fixed delay.
- The URL opened in the browser carries an ID of the preview (`?instance=`). A server started by another Neovim refuses it with "Another preview is using this port" instead of showing a different document.

## [0.7.1] - 2026-10-28

### Changed
- Internal restructuring with no change in behavior: the preview page now lives in `src/preview/` (HTML, CSS and JavaScript as normal files) and the server code is split into modules. The page's CSS and JavaScript are served as `/app.css` and `/app.js`.

## [0.7.0] - 2026-10-28

### Changed
- PDFs look like the preview: `pdf_engine` now defaults to `"browser"`, which prints the preview with the Chrome, Edge, Brave or Chromium already installed (in the background, with a temporary profile). Mermaid diagrams, math, callouts, wiki links, task lists and highlighted code are all rendered, and paper, margin, font, font size, table of contents and heading numbers follow the export settings. Without one of those browsers, Typst is used as before.
- `:MdPreviewInstall` downloads Typst only when PDFs will be made with Typst.

### Added
- Any text file can be previewed. `filetypes` (default `{ "*" }`) selects the file types; Markdown, vimwiki, Quarto, R Markdown, pandoc and MDX are rendered as documents and every other file is shown as highlighted code with line numbers, with scroll sync, click-to-jump, search and the Git diff. Special buffers (help, file explorers, terminals), binary files and files over `max_file_size` (2 MB) are skipped. Export stays Markdown-only.
- `export.browser_path` to choose the browser used for PDFs; `:checkhealth md_preview` shows which one is used.
- The Typst package used for PDFs (cmarker) is bundled with the release and installed by `:MdPreviewInstall`, so Typst no longer downloads it on the first export.

## [0.6.0] - 2026-10-28

### Changed
- `:MdPreviewInstall` downloads the binary from the release that matches the plugin version instead of always the latest one, and falls back to the latest release when that version is not released.

### Fixed
- Formats removed with `export.formats` were still shown in the Export menu: the `hidden` attribute lost to the menu items' `display` style. Any element with `hidden` is now hidden.

### Added
- `nvim-md-preview --version`. The plugin checks it when the preview starts and warns when the binary does not match the plugin version; `:checkhealth md_preview` shows both versions.

## [0.5.0] - 2026-10-28

### Changed
- The Git diff view is on by default (`git_diff = true`). Files outside a Git repository show nothing and no warning.
- README lists every option in tables by category (general, preview, Git diff, export, Vimscript) with defaults and descriptions.

### Fixed
- `export.formats` had no effect. It now limits the formats in the preview's Export menu and in `:MdPreviewExport` completion; `{}` hides the Export button.

## [0.4.1] - 2026-10-28

### Fixed
- The v0.4.0 release is missing the Windows files, so `:MdPreviewInstall` fails on Windows with that version. Releases are now created as drafts and published only after every platform has uploaded its files.

### Changed
- Mermaid, KaTeX and highlight.js are pinned to exact versions (11.17.2, 0.16.47, 11.12.0) and verified against SHA-256 checksums when building. Builds are reproducible and fail if a downloaded file does not match.
- Windows builds use the same `download-libs.sh` as the other platforms.
- Notifications and the export menu no longer use emoji.

## [0.4.0] - 2026-10-28

### Changed
- All Neovim messages, `:checkhealth`, command descriptions, export errors and the preview page are now in English (they were in Japanese).
- README rewritten for Neovim users: hero GIF with Neovim and the preview side by side, comparison with other preview plugins, migration guide from markdown-preview.nvim, recipes, and a screenshot for each feature in dark and light themes.

### Fixed
- The preview toolbar scrolled out of view after one screen on long documents.
- GitHub alerts and callouts were shown in gray instead of their colors.
- Callouts, tables, code and links lost their styling in presentation mode.
- In the Git diff view, a change inside a table was attributed to the heading above it, and a change only inside math showed a mark without the previous version.

## [0.3.0] - 2026-10-28

### Added
- Richer `:MdPreviewDiff`: word-level inline diff (character-level for Japanese), removed paragraphs and the previous version of code/tables/diagrams shown as collapsed boxes, change count with next/previous navigation, scrollbar markers, `+ ~ −` symbols and a legend so changes are clear without color.
- `:MdPreviewDiff {base}` and `git_diff_base` to compare against `staged` or any branch, tag or commit, with completion.
- `git_diff_colors` uses the editor's diff colors when they are readable on the preview theme.
- `browser_keys` option to turn off the preview page's keyboard shortcuts (`t`, `/`, Ctrl+F, `p`), which clash with Vim-style browser extensions such as Vimium and Tridactyl. README FAQ explains the clash and how to exclude the preview in each extension.

## [0.2.0] - 2026-10-28

### Security
- Fixed arbitrary local file reads: `/assets` accepted absolute paths, and the permissive CORS policy let any website read the response while a preview was running. Requests are now limited to files inside the document directory, CORS is removed, and the Host and Origin headers are checked.

### Added
- Block-accurate scroll sync and click-to-jump from the preview to Neovim.
- `:MdPreviewExport` and export of unsaved buffer content; export layout options (paper, margin, font, size, TOC, numbering, Typst template) and relative images in PDF.
- Table of contents panel, in-page search, footnote popups.
- Obsidian callouts, wiki links, opening local Markdown links in Neovim.
- Themes (`auto` / `dark` / `light`) and custom CSS.
- Presentation mode, print preview, `:MdPreviewDiff` for changes since Git `HEAD`.
- LAN mode with per-session token.
- Following the Markdown buffer you switch to.
- `:checkhealth md_preview`, CI, Linux arm64 builds, English README and `:help md_preview`.
- Parallel, non-blocking `:MdPreviewInstall` with progress and compressed pandoc/Typst downloads.

### Changed
- Only changed blocks are re-rendered, and buffer updates are debounced.
- `$...$` and `$$...$$` are parsed as math before Markdown escapes, so TeX like `\,` and `\{` renders correctly; dollar amounts are no longer treated as math.

### Fixed
- Export menu buttons did nothing.
- Export failed for non-ASCII file names and could crash on multibyte error messages.
- Messages from Neovim could be lost when two arrived in one read.

## [0.1.0]

- Initial release.
