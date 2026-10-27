#!/usr/bin/env bash
# download-libs.sh — 開発者・CI専用。cargo build --release の前に実行する。
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/src/embedded_lib"
mkdir -p "$DIR/fonts"

echo "⬇  highlight.js..."
curl -fsSL "https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/highlight.min.js" | gzip -9 > "$DIR/highlight.min.js.gz"
curl -fsSL "https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/styles/github-dark.min.css" | gzip -9 > "$DIR/hljs-dark.min.css.gz"
curl -fsSL "https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/styles/github.min.css" | gzip -9 > "$DIR/hljs-light.min.css.gz"

echo "⬇  Mermaid..."
curl -fsSL "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js" | gzip -9 > "$DIR/mermaid.min.js.gz"

echo "⬇  KaTeX JS..."
curl -fsSL "https://cdn.jsdelivr.net/npm/katex@0.16/dist/katex.min.js" | gzip -9 > "$DIR/katex.min.js.gz"
curl -fsSL "https://cdn.jsdelivr.net/npm/katex@0.16/dist/contrib/auto-render.min.js" | gzip -9 > "$DIR/auto-render.min.js.gz"

echo "⬇  KaTeX CSS (フォントパスを /lib/fonts/ に書き換え後、gzip)..."
curl -fsSL "https://cdn.jsdelivr.net/npm/katex@0.16/dist/katex.min.css" \
  | sed 's|url(fonts/|url(/lib/fonts/|g' \
  | gzip -9 > "$DIR/katex.min.css.gz"

echo "⬇  KaTeX fonts (woff2 はすでに圧縮済みのため非圧縮で埋め込む)..."
for F in \
  KaTeX_AMS-Regular KaTeX_Caligraphic-Bold KaTeX_Caligraphic-Regular \
  KaTeX_Fraktur-Bold KaTeX_Fraktur-Regular KaTeX_Main-Bold KaTeX_Main-BoldItalic \
  KaTeX_Main-Italic KaTeX_Main-Regular KaTeX_Math-BoldItalic KaTeX_Math-Italic \
  KaTeX_SansSerif-Bold KaTeX_SansSerif-Italic KaTeX_SansSerif-Regular \
  KaTeX_Script-Regular KaTeX_Size1-Regular KaTeX_Size2-Regular \
  KaTeX_Size3-Regular KaTeX_Size4-Regular KaTeX_Typewriter-Regular; do
  curl -fsSL -o "$DIR/fonts/${F}.woff2" \
    "https://cdn.jsdelivr.net/npm/katex@0.16/dist/fonts/${F}.woff2"
done

echo ""
echo "✅ 完了。埋め込みサイズ:"
du -sh "$DIR"
echo "次: cargo build --release"
