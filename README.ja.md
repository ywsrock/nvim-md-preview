<div align="center">

# nvim-md-preview

**カーソルに追従し、クリックで戻れる Markdown プレビュー。**<br>
Rust 製の小さなバイナリ 1 つ。Node.js も Deno も不要で、オフラインで動きます。

[![CI](https://github.com/ywsrock/nvim-md-preview/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ywsrock/nvim-md-preview/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ywsrock/nvim-md-preview)](https://github.com/ywsrock/nvim-md-preview/releases/latest)
[![Neovim](https://img.shields.io/badge/Neovim-0.9%2B-57A143?logo=neovim&logoColor=white)](https://neovim.io)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

[English](README.md)

![Neovim で入力するとプレビューが更新され、カーソルを動かすとスクロールし、プレビューをクリックするとカーソルが移動し、:MdPreviewDiff で変更が表示される](docs/hero.gif)

</div>

## nvim-md-preview を選ぶ理由

- **カーソルがプレビューを動かし、プレビューがカーソルを動かす。** カーソルを動かすとプレビューがブロック単位で追従し、プレビューの段落をクリックすると Neovim のその行にカーソルが移ります。
- **入れるのはバイナリ 1 つだけ。** 約 2.5 MB の Rust 製で、Mermaid・KaTeX・highlight.js・フォントを内蔵しています。Node.js も Deno も不要で、インストール後はネットワークも要りません。
- **プレビューだけでは終わらない。** PDF・DOCX・HTML・EPUB への出力、Git との差分の確認、スライドとしての発表まで、同じ画面でできます。

Neovim 0.9 以降が必要です。Vim には対応していません。

## クイックスタート

[lazy.nvim](https://github.com/folke/lazy.nvim) の場合：

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

Markdown ファイルを開いて `:MarkdownPreview` を実行するだけです。

<details>
<summary>vim-plug、packer.nvim</summary>

```vim
" vim-plug: インストール後に一度 :MdPreviewInstall を実行
Plug 'ywsrock/nvim-md-preview'
```

```lua
-- packer.nvim
use { "ywsrock/nvim-md-preview", run = ":MdPreviewInstall" }
```

</details>

`:MdPreviewInstall` は、バイナリと、エクスポート用の pandoc・Typst を並列でダウンロードし、進捗をコマンドラインに表示します。うまく動かないときは `:checkhealth md_preview` を実行してください。

## 他のプラグインとの比較

| | **nvim-md-preview** | [markdown-preview.nvim](https://github.com/iamcco/markdown-preview.nvim) | [peek.nvim](https://github.com/toppair/peek.nvim) |
|---|---|---|---|
| 実行環境 | Rust のバイナリ 1 つ | Node.js アプリ（ビルド済みのものか yarn でビルド） | Deno |
| Vim で使える | いいえ | はい | いいえ |
| カーソルにプレビューが追従 | ブロック単位 | はい | はい |
| プレビューのクリックで Neovim の該当行へ | **はい** | いいえ | いいえ |
| PDF / DOCX / HTML / EPUB への出力 | **内蔵** | ブラウザの印刷 | いいえ |
| 文書の Git 差分表示 | **はい** | いいえ | いいえ |
| 同じファイルでスライド | **はい** | いいえ | いいえ |

[render-markdown.nvim](https://github.com/MeanderingProgrammer/render-markdown.nvim) や [markview.nvim](https://github.com/OXY2DEV/markview.nvim) のようにバッファ内で描画するプラグインは、目的が違います。編集中はそちら、実際のページや図・数式を見たいときや出力したいときは nvim-md-preview、と併用できます。

この表は 2026 年 10 月時点の理解に基づいています。誤りがあれば [Issue](https://github.com/ywsrock/nvim-md-preview/issues) で教えてください。

## 主な機能

### 双方向の同期

スクロールの比率から推測するのではなく、元の行番号の印を使って、カーソルのあるブロックを表示し続けます。プレビューの段落をクリックすると Neovim がその行に移動します。ローカルの `.md` リンクや `[[Wiki リンク]]` も Neovim で開きます。

### オフラインで何でも描画

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/render-dark.png">
  <img alt="Mermaid のフローチャートを含むハンドブックのプレビュー" src="docs/media/render-light.png">
</picture>

Mermaid の図、KaTeX の数式、190 言語のシンタックスハイライトをバイナリに内蔵しているので、ネットワークから何も取得しません。

<table>
<tr>
<td width="50%" valign="top">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/tables-dark.png">
  <img alt="表と数式" src="docs/media/tables-light.png">
</picture>

**表と数式。** 揃え指定つきの GitHub 形式の表、インライン・ブロックの数式。

</td>
<td width="50%" valign="top">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/callouts-dark.png">
  <img alt="コードブロック、callout、タスクリスト" src="docs/media/callouts-light.png">
</picture>

**コード・callout・タスク。** GitHub Alerts と Obsidian の callout（折りたたみも）、タスクリスト、脚注。

</td>
</tr>
</table>

### 何が変わったかを見る

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/diff-dark.png">
  <img alt="Git の HEAD との単語単位の差分" src="docs/media/diff-light.png">
</picture>

`:MdPreviewDiff` で、バッファの内容（未保存の編集を含む）を `HEAD`・ステージ済みの内容・任意のブランチと比較します。変わった語は下線や取り消し線でその場に表示され、表や数式は変更前の内容を折りたたみで、削除された段落もその場所に残ります。ヘッダーの ↑ / ↓ で変更箇所を順に移動でき、スクロールバーの横の印で変更の位置が分かります。

### 長い文書でも迷わない

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/toc-search-dark.png">
  <img alt="目次パネルとページ内検索" src="docs/media/toc-search-light.png">
</picture>

`t` で目次を開き、`/` か Ctrl+F で検索します。スクロールに合わせて、今いる節が強調されます。

### 発表と印刷

<table>
<tr>
<td width="50%" valign="top">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/slides-dark.png">
  <img alt="プレゼンモード" src="docs/media/slides-light.png">
</picture>

**スライド。** `p` で文書をスライドとして発表します。`---` か見出しで区切り、カーソルに追従します。

</td>
<td width="50%" valign="top">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/media/print-dark.png">
  <img alt="A4 の紙面での印刷プレビュー" src="docs/media/print-light.png">
</picture>

**印刷プレビュー。** 改ページの目安つきの A4 の紙面で確認して、ブラウザから印刷します。

</td>
</tr>
</table>

### 出力

`:MdPreviewExport pdf` で、Typst を使ってファイルの隣に PDF を書き出します。用紙サイズ・余白・フォント・目次・独自のテンプレートを指定できます。DOCX・HTML・EPUB は pandoc で出力します。未保存の編集も含まれます。

> [!NOTE]
> PDF の出力では、Mermaid の図・数式・callout はまだ描画されず、元のテキストのまま出力されます。紙に残したいときは印刷プレビューを使ってください。

## レシピ

<details open>
<summary><b>キーの割り当て</b></summary>

```lua
{
  "ywsrock/nvim-md-preview",
  ft = "markdown",
  build = ":MdPreviewInstall",
  config = function() require("md_preview").setup({}) end,
  keys = {
    { "<leader>mp", "<cmd>MarkdownPreviewToggle<cr>", ft = "markdown", desc = "Markdown: プレビュー" },
    { "<leader>md", "<cmd>MdPreviewDiff<cr>", ft = "markdown", desc = "Markdown: Git の差分" },
    {
      "<leader>me",
      function()
        vim.ui.select({ "pdf", "docx", "html", "epub" }, { prompt = "出力形式" }, function(format)
          if format then vim.cmd("MdPreviewExport " .. format) end
        end)
      end,
      ft = "markdown",
      desc = "Markdown: エクスポート",
    },
  },
}
```

</details>

<details>
<summary><b>プレビューを自動で開始する</b></summary>

最初に開いた Markdown ファイルで開始します。`follow_buffer`（既定で有効）により、その後は今いる Markdown バッファにプレビューが切り替わります。

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = "markdown",
  once = true,
  command = "MarkdownPreview",
})
```

</details>

<details>
<summary><b>Obsidian の Vault でメモを書く</b></summary>

`[[Page]]` / `[[Page|表示名]]` のリンクと Obsidian の callout（`> [!info] タイトル`、折りたたみの `> [!todo]-`）は設定なしで使えます。Wiki リンクをクリックすると、大文字小文字・空白・`-`・`_` の違いを無視してノートを探し、Neovim で開きます。見た目は `custom_css = "~/notes/preview.css"` で変えられます。

</details>

<details>
<summary><b>PR のドキュメントをレビューする</b></summary>

ブランチをチェックアウトして `:MdPreviewDiff main` を実行します。`main` から変わった段落すべてに印が付き、プレビューのヘッダーの ↑ / ↓ で順に確認できます。`:MdPreviewDiff staged` でコミット予定の内容、`:MdPreviewDiff off` で表示を消します。

</details>

<details>
<summary><b>会社の書式で PDF を渡す</b></summary>

```lua
require("md_preview").setup({
  export = { paper = "a4", toc = true, numbering = true, template = "~/.config/nvim/report.typ" },
})
```

```typst
// ~/.config/nvim/report.typ
#let template(doc) = {
  set page(header: align(right)[社外秘])
  set text(font: "Noto Sans CJK JP", size: 10pt)
  doc
}
```

あとは `:MdPreviewExport pdf` です。相対パスの画像は文書のフォルダから探します。

</details>

<details>
<summary><b>タブレットで一緒に見る</b></summary>

`host = "0.0.0.0"` にします。プレビューを開始するたびに、使い捨てのトークン付きの LAN の URL が Neovim に表示されます。タブレットでそれを開くと、PC のブラウザと同じようにカーソルに追従します。

</details>

## 使い方

| コマンド | |
|---|---|
| `:MarkdownPreview` / `:MdPreview` | プレビューを開始 |
| `:MarkdownPreviewStop` / `:MdPreviewStop` | 停止 |
| `:MarkdownPreviewToggle` | 開始 / 停止を切り替え |
| `:MdPreviewExport {形式} [path]` | バッファを出力：`pdf` `docx` `html` `epub` `markdown`。`path` を省略するとファイルの隣に保存 |
| `:MdPreviewDiff [基準]` | Git の差分の表示を切り替え。`基準`（`HEAD`、`staged`、ブランチ・タグ・コミット）を指定するとそれと比較。`off` で非表示 |
| `:MdPreviewInstall` | バイナリ・pandoc・Typst をダウンロード / 更新 |

プレビューでは、`t` で目次、`/` か Ctrl+F で検索、`p` でスライド、クリックで該当行へ移動します。ツールバーのボタンでも同じ操作ができます。これらのキーが Vimium などのブラウザ拡張とぶつかる場合は `browser_keys = false` にしてください（[FAQ](#faq) を参照）。

詳しくは `:help md_preview` を見てください。

## 設定

設定はすべて省略できます。よく使うもの：

```lua
require("md_preview").setup({
  theme = "auto",        -- "auto" は OS の設定に従う。"dark" / "light" も可
  auto_open = true,      -- 開始時にブラウザを開く
  export = { paper = "a4" },
})
```

<details>
<summary>すべての設定と既定値</summary>

```lua
require("md_preview").setup({
  port = 9876,            -- 使用中なら次の空きポートを使う
  host = "127.0.0.1",     -- "0.0.0.0" で LAN 内の別端末から見られる（トークンが自動で必須になる）
  auto_open = true,       -- 開始時にブラウザを開く
  scroll_sync = true,     -- カーソルに合わせてプレビューをスクロール
  follow_buffer = true,   -- 別の Markdown バッファに移ったらプレビューを切り替える
  debounce_ms = 80,       -- 入力が止まってから更新するまでの待ち時間
  theme = "auto",         -- "auto"（OS の設定に従う）| "dark" | "light"
  custom_css = "",        -- 組み込みスタイルの後に読み込む CSS ファイル
  wiki_links = true,      -- [[Page]] をリンクにする
  browser_keys = true,    -- プレビュー画面のキーボードショートカット（t, /, Ctrl+F, p）。Vim 系拡張とぶつかる場合は false
  git_diff = false,       -- Git の差分を表示する（:MdPreviewDiff で切り替え）
  git_diff_base = "HEAD", -- 比較の基準: "HEAD" | "staged" | ブランチ・タグ・コミット
  git_diff_colors = true, -- 差分の色をエディタ（gitsigns、Added、DiffAdd など）に合わせる

  export = {
    pdf_engine  = "typst",  -- "typst"。それ以外にすると Typst をダウンロードしない
    docx_engine = "pandoc", -- "pandoc"。それ以外にすると pandoc をダウンロードしない
    pandoc_path = "",       -- 空ならダウンロードしたもの、次に PATH 上のものを使う
    typst_path  = "",
    paper     = "a4",       -- PDF の用紙（Typst の用紙名。例: "us-letter"）
    margin    = "2cm",
    font      = "",         -- 空なら Noto Serif CJK JP など
    font_size = 11,
    toc       = false,      -- 目次（PDF と pandoc の形式）
    numbering = false,      -- 見出し番号
    template  = "",         -- `template` を export した Typst ファイル（レシピを参照）
  },
})
```

Vimscript では `g:md_preview_port` と `g:md_preview_auto_open` も使えます。

</details>

## markdown-preview.nvim から乗り換える

コマンドは同じです。`:MarkdownPreview`、`:MarkdownPreviewStop`、`:MarkdownPreviewToggle` はそのまま使えます。プラグインの指定と `build` を置き換え、設定を次のように移してください。

| markdown-preview.nvim | nvim-md-preview |
|---|---|
| `build = "cd app && npx --yes yarn install"` | `build = ":MdPreviewInstall"` |
| `g:mkdp_port` | `port` |
| `g:mkdp_theme` | `theme` |
| `g:mkdp_markdown_css` | `custom_css` |
| `g:mkdp_open_to_the_world = 1` | `host = "0.0.0.0"`（トークンで保護） |
| `g:mkdp_preview_options.disable_sync_scroll = 1` | `scroll_sync = false` |
| `g:mkdp_auto_start = 1` | [自動で開始するレシピ](#レシピ) |
| `g:mkdp_browser` | まだ対応していません。`auto_open = false` にして URL を自分で開いてください |

## FAQ

### プレビュー画面でショートカットキーが効かない、または新しいタブが開いてしまう

Vimium、Vimium C、Tridactyl、Surfingkeys など、ブラウザに Vim 風のキー操作を追加する拡張機能は、プレビュー画面と同じキーを使っています。

| キー | プレビュー | Vim 系の拡張機能 |
|---|---|---|
| `t` | 目次 | 新しいタブを開く（Vimium、Tridactyl） |
| `/` | 検索 | ページ内検索（Vimium）、クイック検索（Firefox） |
| `p` | プレゼンモード | クリップボードの URL を開く（Vimium、Tridactyl） |
| `f`（スライド中） | 全画面 | リンクヒント |

通常は拡張機能が先にキーを受け取るため、プレビュー画面にはキーが届きません。次のどちらかで対処できます。

- **プレビュー画面のショートカットを無効にする**（ツールバーのボタンで操作します）：

  ```lua
  require("md_preview").setup({ browser_keys = false })
  ```

- **プレビュー画面でだけ拡張機能を無効にする**（プレビュー画面のキーが使えるようになります）：
  - Vimium / Vimium C：オプションの除外 URL のルールに `http://127.0.0.1:*/*` を追加します。拡張機能は使いつつ一部のキーだけ通したい場合は、「Keys」に `t / p` のように指定します。
  - Tridactyl：`:blacklistadd 127.0.0.1` を実行します。
  - Surfingkeys：設定に `settings.blocklistPattern = /127\.0\.0\.1/;` を追加します。

  `port` を変えている場合や LAN モードの場合は、アドレスを読み替えてください。

### 文書がどこかに送られることはありますか？

ありません。サーバーは `127.0.0.1` でのみ待ち受け、必要なものはすべてバイナリに入っています。[セキュリティ](#セキュリティ)を見てください。

## 仕組み

```
Neovim ──msgpack-rpc (stdin/stdout)──▶ Rust バイナリ ──WebSocket──▶ ブラウザ
   ▲                                     │ pulldown-cmark → HTML          │
   └──────── クリック / リンクを開く ─────┴─────────────────────────────────┘
```

Lua 側がバッファの内容（デバウンス済み）とカーソル行を送ります。バイナリは Markdown を元の行番号の印つきの HTML に変換し、`127.0.0.1` でページを配信して、更新を WebSocket で送ります。プレビューでのクリックは同じ経路で Neovim に戻ります。

## セキュリティ

`host` を変えない限り `127.0.0.1` でのみ待ち受けます。`Host` がループバック以外のリクエスト（DNS rebinding）と、他のオリジンからのリクエスト・WebSocket はすべて拒否するので、ブラウザで開いている他のサイトがファイルを読んだり Neovim を操作したりすることはできません。`/assets` は文書のディレクトリ内のファイルだけを返します。LAN モードでは全リクエストにセッションのトークンが必要です。

v0.1.0 までは、プレビュー中に他のサイトからローカルファイルを読み取れる問題がありました。`:MdPreviewInstall` で更新してください。

## 開発に参加する

バグ報告・アイデア・プルリクエストを歓迎します。ソースからのビルド、テスト、リリースの流れは [CONTRIBUTING.md](CONTRIBUTING.md) を見てください。

```bash
git clone https://github.com/ywsrock/nvim-md-preview
cd nvim-md-preview
bash download-libs.sh && cargo build --release
```

## 謝辞

- [selimacerbas/mdkite.nvim](https://github.com/selimacerbas/mdkite.nvim) — プレビュー画面のスタイル・Mermaid の描画・テーマ設計を参考にしました。
- [iamcco/markdown-preview.nvim](https://github.com/iamcco/markdown-preview.nvim) — インストール方式・コマンド名・設定インターフェースを参考にしました。

## ライセンス

MIT
