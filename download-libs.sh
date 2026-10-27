#!/usr/bin/env bash
# download-libs.sh — 開発者・CI専用。cargo build --release の前に実行する。
# ユーザーはこれを実行する必要はない（バイナリに埋め込み済み）
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/src/embedded_lib"
mkdir -p "$DIR/fonts"

echo "⬇  highlight.js..."
curl -fsSL -o "$DIR/highlight.min.js"   "https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/highlight.min.js"
curl -fsSL -o "$DIR/hljs-dark.min.css"  "https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/styles/github-dark.min.css"
curl -fsSL -o "$DIR/hljs-light.min.css" "https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/styles/github.min.css"

echo "⬇  Mermaid (UMD版 — 単一ファイル、チャンク分割なし)..."
curl -fsSL -o "$DIR/mermaid.min.js" "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js"

echo "⬇  KaTeX JS..."
curl -fsSL -o "$DIR/katex.min.js"      "https://cdn.jsdelivr.net/npm/katex@0.16/dist/katex.min.js"
curl -fsSL -o "$DIR/auto-render.min.js" "https://cdn.jsdelivr.net/npm/katex@0.16/dist/contrib/auto-render.min.js"

echo "⬇  KaTeX CSS (フォントパスを /lib/fonts/ に書き換え)..."
curl -fsSL "https://cdn.jsdelivr.net/npm/katex@0.16/dist/katex.min.css" \
  | sed 's|url(fonts/|url(/lib/fonts/|g' \
  > "$DIR/katex.min.css"

echo "⬇  KaTeX fonts..."
for F in \
  KaTeX_AMS-Regular KaTeX_Caligraphic-Bold KaTeX_Caligraphic-Regular \
  KaTeX_Fraktur-Bold KaTeX_Fraktur-Regular KaTeX_Main-Bold KaTeX_Main-BoldItalic \
  KaTeX_Main-Italic KaTeX_Main-Regular KaTeX_Math-BoldItalic KaTeX_Math-Italic \
  KaTeX_SansSerif-Bold KaTeX_SansSerif-Italic KaTeX_SansSerif-Regular \
  KaTeX_Script-Regular KaTeX_Size1-Regular KaTeX_Size2-Regular \
  KaTeX_Size3-Regular KaTeX_Size4-Regular KaTeX_Typewriter-Regular; do
  echo "   $F.woff2"
  curl -fsSL -o "$DIR/fonts/${F}.woff2" \
    "https://cdn.jsdelivr.net/npm/katex@0.16/dist/fonts/${F}.woff2"
done

echo "✅ src/embedded_lib/ に保存完了。次: cargo build --release"
