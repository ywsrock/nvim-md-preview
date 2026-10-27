# Changelog

All notable changes are documented here. This project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
