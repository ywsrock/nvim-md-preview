<div align="center">

# nvim-md-preview

**The Markdown preview that follows your cursor, and lets you click back.**<br>
One small Rust binary. No Node.js, no Deno, works offline.

[![CI](https://github.com/ywsrock/nvim-md-preview/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ywsrock/nvim-md-preview/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ywsrock/nvim-md-preview)](https://github.com/ywsrock/nvim-md-preview/releases/latest)
[![Neovim](https://img.shields.io/badge/Neovim-0.9%2B-57A143?logo=neovim&logoColor=white)](https://neovim.io)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

[日本語](README.ja.md)

![Typing in Neovim updates the preview, moving the cursor scrolls it, clicking the preview moves the cursor, and :MdPreviewDiff shows the changes](docs/hero.gif)

</div>

## Why nvim-md-preview

- **Your cursor drives the preview, and the preview drives your cursor.** The preview scrolls block by block as you move, and clicking any paragraph puts the cursor on that line in Neovim.
- **Nothing to install but one binary.** About 2.5 MB of Rust with Mermaid, KaTeX, highlight.js and fonts built in. No Node.js or Deno, no network needed once installed.
- **It goes beyond previewing.** Export to PDF, DOCX, HTML or EPUB, review your changes against Git, and present the document as slides, all from the same window.

Requires Neovim 0.9 or later. Vim is not supported.

## Quick start

With [lazy.nvim](https://github.com/folke/lazy.nvim):

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

Open a Markdown file and run `:MarkdownPreview`. That's it.

<details>
<summary>vim-plug, packer.nvim</summary>

```vim
" vim-plug: then run :MdPreviewInstall once
Plug 'ywsrock/nvim-md-preview'
```

```lua
-- packer.nvim
use { "ywsrock/nvim-md-preview", run = ":MdPreviewInstall" }
```

</details>

`:MdPreviewInstall` downloads the binary that matches the plugin version (plus pandoc and Typst for export) in parallel with progress on the command line. If you update the plugin without a `build` hook, the preview warns when the binary is out of date; run `:MdPreviewInstall` again. Something wrong? Run `:checkhealth md_preview`.

## How it compares

| | **nvim-md-preview** | [markdown-preview.nvim](https://github.com/iamcco/markdown-preview.nvim) | [peek.nvim](https://github.com/toppair/peek.nvim) |
|---|---|---|---|
| Runtime | Single Rust binary | Node.js app (prebuilt bundle or build with yarn) | Deno |
| Works in Vim | No | Yes | No |
| Preview follows the cursor | Block by block | Yes | Yes |
| Click the preview to jump in Neovim | **Yes** | No | No |
| Export to PDF / DOCX / HTML / EPUB | **Built in** | Browser print | No |
| Git diff of the document | **Yes** | No | No |
| Slides from the same file | **Yes** | No | No |

In-buffer renderers such as [render-markdown.nvim](https://github.com/MeanderingProgrammer/render-markdown.nvim) and [markview.nvim](https://github.com/OXY2DEV/markview.nvim) solve a different problem. Use them while editing and nvim-md-preview when you want the real page, diagrams and math, or an exported file. They work well together.

This table reflects our understanding as of October 2026. If something is wrong, please [open an issue](https://github.com/ywsrock/nvim-md-preview/issues).

## Highlights

### Two-way sync

The preview keeps the block under your cursor in view, using source-line markers instead of guessing from the scroll ratio. Click a paragraph in the preview and Neovim moves there. Local `.md` links and `[[wiki links]]` open in Neovim too.

### Everything renders, offline

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/render-dark.png">
  <img alt="Preview of a handbook with a Mermaid flowchart" src="docs/media/render-light.png">
</picture>

Mermaid diagrams, KaTeX math and syntax highlighting for 190 languages are built into the binary, so nothing is fetched from the network.

<table>
<tr>
<td width="50%" valign="top">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/tables-dark.png">
  <img alt="Table and display math" src="docs/media/tables-light.png">
</picture>

**Tables and math.** GitHub-flavored tables with alignment, inline and display math.

</td>
<td width="50%" valign="top">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/callouts-dark.png">
  <img alt="Code block, callouts and task list" src="docs/media/callouts-light.png">
</picture>

**Code, callouts and tasks.** GitHub alerts and Obsidian callouts, including foldable ones, task lists and footnotes.

</td>
</tr>
</table>

### See what changed

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/diff-dark.png">
  <img alt="Word-level diff: an added sentence underlined, a removed callout and a new paragraph" src="docs/media/diff-light.png">
</picture>

Changes against `HEAD` are shown as soon as you edit a file in a Git repository, unsaved edits included. `:MdPreviewDiff main` compares with a branch instead, `:MdPreviewDiff staged` with the index, and `:MdPreviewDiff off` hides it. Changed words are underlined or struck through in place, tables and math show the previous version in a collapsed box, removed paragraphs stay visible, and ↑ / ↓ in the header jump from change to change. The markers beside the scrollbar show where the changes are.

### Find your way in long documents

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/toc-search-dark.png">
  <img alt="Table of contents panel and in-page search" src="docs/media/toc-search-light.png">
</picture>

Open the table of contents with `t` and search with `/` or Ctrl+F. The current section is highlighted as you scroll.

### Present and print

<table>
<tr>
<td width="50%" valign="top">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/slides-dark.png">
  <img alt="Presentation mode" src="docs/media/slides-light.png">
</picture>

**Slides.** Press `p` to present the document, split on `---` or on headings. The slide follows your cursor.

</td>
<td width="50%" valign="top">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/print-dark.png">
  <img alt="Print preview on an A4 sheet" src="docs/media/print-light.png">
</picture>

**Print preview.** See the document on an A4 sheet with page-break guides, then print from the browser.

</td>
</tr>
</table>

### Export

`:MdPreviewExport pdf` writes a PDF next to your file using Typst, with paper size, margins, fonts, a table of contents and your own template. DOCX, HTML and EPUB go through pandoc. Unsaved edits are included.

> [!NOTE]
> PDF export does not render Mermaid diagrams, math or callouts yet; they appear as their source text. Use the print preview when you need them on paper.

## Recipes

<details open>
<summary><b>Keymaps</b></summary>

```lua
{
  "ywsrock/nvim-md-preview",
  ft = "markdown",
  build = ":MdPreviewInstall",
  config = function() require("md_preview").setup({}) end,
  keys = {
    { "<leader>mp", "<cmd>MarkdownPreviewToggle<cr>", ft = "markdown", desc = "Markdown: preview" },
    { "<leader>md", "<cmd>MdPreviewDiff<cr>", ft = "markdown", desc = "Markdown: Git diff" },
    {
      "<leader>me",
      function()
        vim.ui.select({ "pdf", "docx", "html", "epub" }, { prompt = "Export as" }, function(format)
          if format then vim.cmd("MdPreviewExport " .. format) end
        end)
      end,
      ft = "markdown",
      desc = "Markdown: export",
    },
  },
}
```

</details>

<details>
<summary><b>Start the preview automatically</b></summary>

Start it with the first Markdown file you open. With `follow_buffer` (on by default), the preview then switches to whichever Markdown buffer you are in.

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = "markdown",
  once = true,
  command = "MarkdownPreview",
})
```

</details>

<details>
<summary><b>Notes in an Obsidian vault</b></summary>

`[[Page]]` and `[[Page|label]]` links and Obsidian callouts (`> [!info] Title`, foldable `> [!todo]-`) work out of the box. Clicking a wiki link opens the note in Neovim, matching names regardless of case, spaces, `-` and `_`. Add your own look with `custom_css = "~/notes/preview.css"`.

</details>

<details>
<summary><b>Review a pull request's docs</b></summary>

Check out the branch and run `:MdPreviewDiff main`. Every changed paragraph is marked against `main`, with ↑ / ↓ in the preview header to walk through them. `:MdPreviewDiff staged` shows what you are about to commit, and `:MdPreviewDiff off` hides it again.

</details>

<details>
<summary><b>Hand off a PDF with your company's layout</b></summary>

```lua
require("md_preview").setup({
  export = { paper = "a4", toc = true, numbering = true, template = "~/.config/nvim/report.typ" },
})
```

```typst
// ~/.config/nvim/report.typ
#let template(doc) = {
  set page(header: align(right)[Internal])
  set text(font: "Inter", size: 10pt)
  doc
}
```

Then `:MdPreviewExport pdf`. Relative images are resolved from the document's folder.

</details>

<details>
<summary><b>Follow along on a tablet</b></summary>

Set `host = "0.0.0.0"`. Each time the preview starts, Neovim shows LAN URLs with a one-time token. Open one on your tablet and it follows your cursor like the desktop browser does.

</details>

## Usage

| Command | |
|---|---|
| `:MarkdownPreview` / `:MdPreview` | Start the preview |
| `:MarkdownPreviewStop` / `:MdPreviewStop` | Stop it |
| `:MarkdownPreviewToggle` | Start or stop |
| `:MdPreviewExport {format} [path]` | Export the buffer: `pdf`, `docx`, `html`, `epub`, `markdown`. Saved next to the file unless `path` is given |
| `:MdPreviewDiff [base]` | Toggle the Git diff, or compare with `base` (`HEAD`, `staged`, a branch, tag or commit). `off` hides it |
| `:MdPreviewInstall` | Download or update the binary, pandoc and Typst |

In the preview: `t` table of contents, `/` or Ctrl+F search, `p` slides, click to jump. The toolbar buttons do the same. If these keys clash with a browser extension such as Vimium, set `browser_keys = false` (see [FAQ](#faq)).

Full reference: `:help md_preview`.

## Configuration

Every option is optional. Call `setup()` with only what you want to change:

```lua
require("md_preview").setup({
  theme = "light",
  git_diff_base = "main",
  export = { paper = "us-letter", toc = true },
})
```

### General

| Option | Default | Description |
|---|---|---|
| `port` | `9876` | Port of the preview server. If it is taken, the next free port is used. |
| `host` | `"127.0.0.1"` | Address to listen on. `"0.0.0.0"` lets other devices on your LAN open the preview; a token is then required (see "Follow along on a tablet" in [Recipes](#recipes)). |
| `auto_open` | `true` | Open the browser when the preview starts. |
| `binary` | Downloaded copy in `stdpath("data")/nvim-md-preview/` | Path to the preview binary. Set it when you build from source. |

### Preview

| Option | Default | Description |
|---|---|---|
| `scroll_sync` | `true` | Scroll the preview to the block under the cursor. |
| `follow_buffer` | `true` | Switch the preview when you enter another Markdown buffer. |
| `debounce_ms` | `80` | Milliseconds to wait after typing before the preview updates. |
| `theme` | `"auto"` | `"auto"` follows the OS, or `"dark"` / `"light"`. A theme chosen with the button in the preview is remembered and takes precedence. |
| `custom_css` | `""` | Path to a CSS file loaded after the built-in styles, so it can override them. |
| `wiki_links` | `true` | Render `[[Page]]` and `[[Page\|label]]` as links that open in Neovim. |
| `browser_keys` | `true` | Keyboard shortcuts in the preview (`t`, `/`, Ctrl+F, `p`). Set `false` if they clash with a browser extension such as Vimium (see [FAQ](#faq)). |

### Git diff

| Option | Default | Description |
|---|---|---|
| `git_diff` | `true` | Show changes against `git_diff_base` in the preview. Toggle with `:MdPreviewDiff`. Files outside a Git repository show nothing. |
| `git_diff_base` | `"HEAD"` | What to compare against: `"HEAD"`, `"staged"` (the index), or a branch, tag or commit. `:MdPreviewDiff <base>` changes it for the session. |
| `git_diff_colors` | `true` | Use the editor's diff colors (`GitSignsAdd`, `Added`, `DiffAdd`, ...). Colors that are hard to read on the preview's theme fall back to green, yellow and red. |

### Export (`export = { ... }`)

| Option | Default | Description |
|---|---|---|
| `formats` | `{ "pdf", "docx", "html", "epub", "markdown" }` | Formats offered in the preview's Export menu and in `:MdPreviewExport` completion. `{}` hides the Export button. |
| `pdf_engine` | `"typst"` | `"typst"` makes `:MdPreviewInstall` download Typst for PDF export. Any other value skips it. |
| `docx_engine` | `"pandoc"` | `"pandoc"` makes `:MdPreviewInstall` download pandoc for DOCX, HTML and EPUB. Any other value skips it. |
| `pandoc_path` | `""` | Path to pandoc. Empty uses the downloaded copy, then `pandoc` on your `PATH`. |
| `typst_path` | `""` | Path to Typst. Empty uses the downloaded copy, then `typst` on your `PATH`. |
| `paper` | `"a4"` | PDF paper size. Any Typst paper name, such as `"us-letter"` or `"a5"`. |
| `margin` | `"2cm"` | PDF page margin. |
| `font` | `""` | PDF body font. Empty uses Noto Serif CJK JP with fallbacks. |
| `font_size` | `11` | PDF body font size in points. |
| `toc` | `false` | Add a table of contents (PDF, DOCX, HTML, EPUB). |
| `numbering` | `false` | Number headings (PDF, DOCX, HTML, EPUB). |
| `template` | `""` | Typst file that exports a `template` function, applied to the whole PDF (see [Recipes](#recipes)). |

### Vimscript

Without `setup()`, these variables are read when the plugin loads:

| Variable | Same as |
|---|---|
| `g:md_preview_port` | `port` |
| `g:md_preview_auto_open` | `auto_open` (`0` turns it off) |

<details>
<summary>All options with their defaults, ready to copy</summary>

```lua
require("md_preview").setup({
  binary = vim.fn.stdpath("data") .. "/nvim-md-preview/nvim-md-preview",
  port = 9876,
  host = "127.0.0.1",
  auto_open = true,

  scroll_sync = true,
  follow_buffer = true,
  debounce_ms = 80,
  theme = "auto",
  custom_css = "",
  wiki_links = true,
  browser_keys = true,

  git_diff = true,
  git_diff_base = "HEAD",
  git_diff_colors = true,

  export = {
    formats = { "pdf", "docx", "html", "epub", "markdown" },
    pdf_engine = "typst",
    docx_engine = "pandoc",
    pandoc_path = "",
    typst_path = "",
    paper = "a4",
    margin = "2cm",
    font = "",
    font_size = 11,
    toc = false,
    numbering = false,
    template = "",
  },
})
```

</details>

## Coming from markdown-preview.nvim

The commands are the same: `:MarkdownPreview`, `:MarkdownPreviewStop` and `:MarkdownPreviewToggle` keep working. Replace the plugin spec and `build` step, then map your settings:

| markdown-preview.nvim | nvim-md-preview |
|---|---|
| `build = "cd app && npx --yes yarn install"` | `build = ":MdPreviewInstall"` |
| `g:mkdp_port` | `port` |
| `g:mkdp_theme` | `theme` |
| `g:mkdp_markdown_css` | `custom_css` |
| `g:mkdp_open_to_the_world = 1` | `host = "0.0.0.0"` (protected by a token) |
| `g:mkdp_preview_options.disable_sync_scroll = 1` | `scroll_sync = false` |
| `g:mkdp_auto_start = 1` | The [auto-start recipe](#recipes) |
| `g:mkdp_browser` | Not supported yet. Set `auto_open = false` and open the URL yourself |

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

### Does it send my documents anywhere?

No. The server listens on `127.0.0.1` only, and everything it needs is inside the binary. See [Security](#security).

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

## Contributing

Bug reports, ideas and pull requests are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for building from source, tests and the release flow.

```bash
git clone https://github.com/ywsrock/nvim-md-preview
cd nvim-md-preview
bash download-libs.sh && cargo build --release
```

## Acknowledgements

- [selimacerbas/mdkite.nvim](https://github.com/selimacerbas/mdkite.nvim) for the preview styling, Mermaid rendering and theme design
- [iamcco/markdown-preview.nvim](https://github.com/iamcco/markdown-preview.nvim) for the install flow, command names and configuration interface

## License

MIT
