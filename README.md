# nvim-md-preview

Live Markdown preview for Neovim. A single Rust binary, fully offline, no Node.js or Deno.

[日本語](README.ja.md)

![demo](docs/demo.gif)

- **Accurate two-way sync**: the preview follows your cursor block by block, and clicking a paragraph in the preview jumps to that line in Neovim.
- **Fast on large documents**: only changed blocks are re-rendered; diagrams and scroll position are kept while you type.
- **Works offline**: Mermaid, KaTeX, highlight.js and fonts are embedded in the binary.
- **Export built in**: PDF (Typst), DOCX / HTML / EPUB (pandoc) and Markdown, from the browser or `:MdPreviewExport`. Unsaved edits are included.
- **Made for long documents**: table of contents, in-page search, footnote popups, presentation mode and print preview.
- **Secure by default**: listens on `127.0.0.1` only and rejects cross-site requests. LAN sharing needs a per-session token.

## Features

| | |
|---|---|
| Rendering | GFM tables, task lists, footnotes, strikethrough, Mermaid, KaTeX math (`$...$`, `$$...$$`), syntax highlighting (190 languages), emoji `:smile:`, YAML front matter |
| Callouts | GitHub alerts (`> [!NOTE]`) and Obsidian callouts (`> [!info] Title`, foldable `> [!todo]-`) |
| Links | Wiki links `[[Page]]` / `[[Page\|label]]`; local `.md` links and wiki links open in Neovim |
| Sync | Block-accurate scroll sync, click-to-jump, follows the Markdown buffer you switch to |
| Navigation | TOC panel (`t`), search (`/` or Ctrl+F), heading anchors |
| Modes | Presentation mode (`p`, slides split on `---`), print preview with A4 page guides |
| Git | `:MdPreviewDiff` marks blocks changed since `HEAD` |
| Export | PDF / DOCX / HTML / EPUB / Markdown with paper size, margins, font, TOC, numbering and custom Typst templates |
| Look | Dark / light / auto theme, custom CSS |
| Platforms | Linux (x86_64, arm64), macOS (Intel, Apple Silicon), Windows, WSL |

## Requirements

- Neovim 0.9 or later
- `curl` (or `wget` / PowerShell) to download the binary. `tar` is used to unpack pandoc and Typst and is preinstalled on macOS, Linux and Windows 10+.

## Installation

### lazy.nvim

```lua
{
  "ywsrock/nvim-md-preview",
  ft = "markdown",
  build = ":MdPreviewInstall",
  config = function()
    require("md_preview").setup({})
  end,
}
```

### vim-plug

```vim
Plug 'ywsrock/nvim-md-preview'
```

Then run `:MdPreviewInstall` once.

### packer.nvim

```lua
use { "ywsrock/nvim-md-preview", run = ":MdPreviewInstall" }
```

`:MdPreviewInstall` downloads the preview binary plus pandoc and Typst in parallel, showing progress on the command line. If you skip it, the binary is downloaded the first time you run `:MarkdownPreview`.

Run `:checkhealth md_preview` if something does not work.

## Usage

Open a Markdown file and run `:MarkdownPreview`.

| Command | |
|---|---|
| `:MarkdownPreview` / `:MdPreview` | Start the preview |
| `:MarkdownPreviewStop` / `:MdPreviewStop` | Stop it |
| `:MarkdownPreviewToggle` | Start or stop |
| `:MdPreviewExport {format} [path]` | Export the current buffer: `pdf`, `docx`, `html`, `epub`, `markdown`. Without `path`, it is saved next to the file |
| `:MdPreviewDiff` | Show or hide changes since Git `HEAD` |
| `:MdPreviewInstall` | Download or update the binary, pandoc and Typst |

Keys in the browser:

| Key | |
|---|---|
| `t` | Toggle the table of contents |
| `/`, Ctrl+F | Search (Enter / Shift+Enter to move, Esc to close) |
| `p` | Presentation mode (arrows to move, `f` fullscreen, Esc to exit) |
| Click a paragraph | Jump to that line in Neovim |

The toolbar buttons do the same. If these keys clash with a browser extension, set `browser_keys = false` (see [FAQ](#faq)).

## Configuration

Defaults are shown. Pass only what you want to change.

```lua
require("md_preview").setup({
  port = 9876,            -- the next free port is used if this one is taken
  host = "127.0.0.1",     -- "0.0.0.0" to view from other devices on your LAN (a token is required automatically)
  auto_open = true,       -- open the browser when the preview starts
  scroll_sync = true,     -- scroll the preview with the cursor
  follow_buffer = true,   -- switch the preview when you enter another Markdown buffer
  debounce_ms = 80,       -- wait this long after typing before updating
  theme = "auto",         -- "auto" (follow the OS) | "dark" | "light"
  custom_css = "",        -- path to a CSS file loaded after the built-in styles
  wiki_links = true,      -- render [[Page]] links
  browser_keys = true,    -- keyboard shortcuts in the preview (t, /, Ctrl+F, p); false if they clash with a Vim extension
  git_diff = false,       -- mark blocks changed since HEAD (toggle with :MdPreviewDiff)

  export = {
    pdf_engine  = "typst",  -- "typst"; anything else skips downloading Typst
    docx_engine = "pandoc", -- "pandoc"; anything else skips downloading pandoc
    pandoc_path = "",       -- empty: use the downloaded copy, then PATH
    typst_path  = "",
    paper     = "a4",       -- PDF paper size (any Typst paper name, e.g. "us-letter")
    margin    = "2cm",
    font      = "",         -- empty: Noto Serif CJK JP and fallbacks
    font_size = 11,
    toc       = false,      -- table of contents (PDF and pandoc formats)
    numbering = false,      -- numbered headings
    template  = "",         -- Typst file that exports `template` (see below)
  },
})
```

Vimscript users can set `g:md_preview_port` and `g:md_preview_auto_open`.

### Custom PDF template

`export.template` points to a Typst file that exports a `template` function. It is applied as a show rule around the document:

```typst
// ~/.config/nvim/md-template.typ
#let template(doc) = {
  set page(paper: "a4", header: align(right)[Internal])
  set text(font: "Inter", size: 10pt)
  doc
}
```

### Viewing from a tablet or another computer

Set `host = "0.0.0.0"`. Each time the preview starts, a random token is generated and the LAN URLs are shown in Neovim. Only those URLs (or a browser that already opened one) can access the preview.

## FAQ

### The keyboard shortcuts in the preview do nothing, or open a new tab instead

Browser extensions that add Vim-style keys, such as Vimium, Vimium C, Tridactyl and Surfingkeys, use the same keys as the preview:

| Key | Preview | Vim-style extensions |
|---|---|---|
| `t` | Table of contents | Open a new tab (Vimium, Tridactyl) |
| `/` | Search | Find in page (Vimium), quick find (Firefox) |
| `p` | Presentation mode | Open the clipboard URL (Vimium, Tridactyl) |
| `f` (in slides) | Fullscreen | Link hints |

The extension usually receives the key first, so the preview never sees it. You can do either of these:

- **Turn off the preview's shortcuts** and use the toolbar buttons instead:

  ```lua
  require("md_preview").setup({ browser_keys = false })
  ```

- **Turn off the extension on the preview page** so the preview's keys work:
  - Vimium / Vimium C: in the options, add an excluded URL rule for `http://127.0.0.1:*/*`. To keep the extension and only pass some keys through, put them in "Keys", for example `t / p`.
  - Tridactyl: run `:blacklistadd 127.0.0.1`.
  - Surfingkeys: add `settings.blocklistPattern = /127\.0\.0\.1/;` to your settings.

  If you changed `port` or use LAN mode, adjust the address accordingly.

## How it works

```
Neovim ──msgpack-rpc (stdin/stdout)──▶ Rust binary ──WebSocket──▶ Browser
   ▲                                     │ pulldown-cmark → HTML           │
   └──────────── click / open link ──────┴──────────────────────────────────┘
```

The Lua side sends the buffer (debounced) and the cursor line. The binary converts Markdown to HTML with source-line markers, serves the page and assets on `127.0.0.1`, and pushes updates over a WebSocket. Clicks in the preview travel back the same way.

## Security

The server binds to `127.0.0.1` unless you set `host`. It rejects requests whose `Host` is not a loopback name (DNS rebinding) and any cross-origin request or WebSocket, so other websites open in your browser cannot read your files or control Neovim. `/assets` only serves files inside the document's directory. In LAN mode every request needs the session token.

Versions up to v0.1.0 allowed other websites to read local files while a preview was running. Update with `:MdPreviewInstall`.

## Building from source

```bash
git clone https://github.com/ywsrock/nvim-md-preview
cd nvim-md-preview
bash download-libs.sh      # fetch the JS/CSS libraries embedded in the binary
cargo build --release
```

Then point the plugin at the binary:

```lua
require("md_preview").setup({ binary = "/path/to/nvim-md-preview/target/release/nvim-md-preview" })
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for tests and the development workflow.

## Acknowledgements

- [selimacerbas/mdkite.nvim](https://github.com/selimacerbas/mdkite.nvim) for the preview styling, Mermaid rendering and theme design
- [iamcco/markdown-preview.nvim](https://github.com/iamcco/markdown-preview.nvim) for the install flow, command names and configuration interface

## License

MIT
