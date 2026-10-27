# nvim-md-preview

Neovim 用 Markdown プレビュープラグイン。Rust 製のローカル HTTP サーバーでブラウザにリアルタイムプレビューを提供します。

## 特徴

- ⚡ リアルタイムプレビュー（WebSocket）
- 🎨 Mermaid ダイアグラム（フローチャート、シーケンス図など）
- 📐 KaTeX 数式レンダリング（`$...$`, `$$...$$`）
- 🖍️ シンタックスハイライト（highlight.js、190言語対応）
- 🌙 ダーク / ライトテーマ切り替え
- 📜 スクロール同期（カーソル追従）
- 🔗 見出しアンカーリンク
- ⚠️ GitHub Alerts（`> [!NOTE]` など）
- 😊 絵文字（`:smile:` → 😊）
- 📦 YAML front matter パネル表示
- 🖼️ 相対パス画像の自動解決
- 📡 オフライン完全動作（JS/CSS をバイナリに埋め込み済み）
- 🔌 ポート競合時は自動で空きポートを使用
- 📤 エクスポート（PDF / DOCX / HTML / EPUB / Markdown）

## 動作環境

| OS | 対応 |
|---|---|
| Linux | ✅ |
| macOS (Intel / Apple Silicon) | ✅ |
| Windows ネイティブ | ✅ |
| Windows (WSL) | ✅ |

---

## インストール

### Vundle

`~/.config/nvim/plugin/plugins.vim` に追加:

```vim
Plugin 'ywsrock/nvim-md-preview'
```

`:PluginInstall` 後に実行:

```vim
:MdPreviewInstall
```

または初回 `:MarkdownPreview` 実行時に自動ダウンロードします。

### lazy.nvim

```lua
{
  "ywsrock/nvim-md-preview",
  ft = "markdown",
  build = ":MdPreviewInstall",
  config = function()
    require("md_preview").setup()
  end,
}
```

### vim-plug

```vim
Plug 'ywsrock/nvim-md-preview'
```

`:PlugInstall` 後:

```vim
:MdPreviewInstall
```

### packer.nvim

```lua
use {
  "ywsrock/nvim-md-preview",
  run = ":MdPreviewInstall",
}
```

---

## 使い方

Markdown ファイルを開いて:

```vim
:MarkdownPreview        " プレビュー開始
:MarkdownPreviewStop    " プレビュー停止
:MarkdownPreviewToggle  " 開始/停止トグル

" 短縮コマンドも使用可能
:MdPreview
:MdPreviewStop
:MdPreviewInstall       " バイナリを手動ダウンロード
```

---

## 設定

### Lua (init.lua)

```lua
require("md_preview").setup({
  port      = 9876,   -- HTTP サーバーのポート番号（競合時は自動で別ポートを使用）
  auto_open = true,   -- プレビュー開始時に自動でブラウザを開く

  -- エクスポート設定
  export = {
    -- PDF エンジン: "typst"（高品質・推奨）
    pdf_engine  = "typst",
    -- DOCX/HTML/EPUB エンジン: "pandoc"（推奨）
    docx_engine = "pandoc",

    -- バイナリパスを手動指定（省略時は自動ダウンロード先を使用）
    pandoc_path = "",
    typst_path  = "",
  },
})
```

### Vimscript (init.vim)

```vim
let g:md_preview_port      = 9876   " ポート番号
let g:md_preview_auto_open = 1      " 自動でブラウザを開く（0 で無効）
```

---

## エクスポート機能

プレビュー画面のツールバーにある **Export ▼** ボタンから各形式でエクスポートできます。

| 形式 | エンジン | 品質 | 必要なもの |
|---|---|---|---|
| **PDF** | Typst | ⭐⭐⭐⭐⭐ | `:MdPreviewInstall` で自動インストール |
| **DOCX** | pandoc | ⭐⭐⭐⭐⭐ | `:MdPreviewInstall` で自動インストール |
| **HTML** | pandoc | ⭐⭐⭐⭐⭐ | `:MdPreviewInstall` で自動インストール |
| **EPUB** | pandoc | ⭐⭐⭐⭐ | `:MdPreviewInstall` で自動インストール |
| **Markdown** | なし | — | 不要 |

### インストール（初回のみ）

`:MdPreviewInstall` を実行すると以下が自動でダウンロードされます：

```
# stdpath("data")/nvim-md-preview/ （OS ごとに場所が異なる）
# Linux:   ~/.local/share/nvim/nvim-md-preview/
# macOS:   ~/Library/Application Support/nvim/nvim-md-preview/
# Windows: %LOCALAPPDATA%\nvim-data\nvim-md-preview\

nvim-md-preview    ← プレビューバイナリ
pandoc             ← DOCX/HTML/EPUB 用（~30MB）
typst              ← PDF 用（~30MB）
```

### バイナリパスを手動指定する場合

```lua
require("md_preview").setup({
  export = {
    pandoc_path = "/usr/local/bin/pandoc",
    typst_path  = "/usr/local/bin/typst",
  },
})
```

---

## Mermaid ダイアグラム

````markdown
```mermaid
graph TD
    A[開始] --> B{条件}
    B -- Yes --> C[処理]
    B -- No  --> D[終了]
```
````

## 数式 (KaTeX)

```markdown
インライン: $E = mc^2$

ブロック:
$$
\int_{-\infty}^{\infty} e^{-x^2} dx = \sqrt{\pi}
$$
```

## GitHub Alerts

```markdown
> [!NOTE]
> 補足情報です。

> [!WARNING]
> 注意が必要な情報です。

> [!TIP]
> ヒントです。
```

---

## 仕組み

```
Neovim (バッファ変更)
  ↓ RPC (stdin/stdout msgpack)
Rust バイナリ
  ├── pulldown-cmark で Markdown → HTML 変換
  ├── HTTP/WebSocket サーバー (axum)
  └── JS/CSS/フォントをバイナリ内蔵（オフライン対応）
       ↓ WebSocket
ブラウザ（リアルタイム更新）
```

---

## 開発者向け（ソースからビルド）

```bash
git clone https://github.com/ywsrock/nvim-md-preview
cd nvim-md-preview

# JS ライブラリをダウンロード（初回のみ）
bash download-libs.sh

# ビルド
cargo build --release
```

`init.vim` にバイナリパスを設定:

```vim
lua require("md_preview").setup({
\  binary = vim.fn.expand("~/.vim/bundle/nvim-md-preview/target/release/nvim-md-preview"),
\ })
```

---

## ライセンス

MIT

---

## 備考

本プラグインの開発にあたり、以下のプラグインを参考にしました。

- [selimacerbas/mdkite.nvim](https://github.com/selimacerbas/mdkite.nvim) — プレビュー画面のスタイル・Mermaid レンダリング・テーマ設計を参考にしました。
- [iamcco/markdown-preview.nvim](https://github.com/iamcco/markdown-preview.nvim) — インストール方式・コマンド設計・設定インターフェースを参考にしました。
