#!/usr/bin/env bash
# バイナリに埋め込む JavaScript / CSS / フォントを src/embedded_lib/ に用意する。
# cargo build の前に一度実行する（CI とリリースも同じスクリプトを使う）。
#
# 取得するファイルはバージョンを固定し、SHA-256 で照合する。一致しなければ失敗する。
# バージョンを上げるときは下の *_VERSION を変え、
#   bash download-libs.sh --update-checksums
# で表示される値で CHECKSUMS を置き換える。
set -euo pipefail

MERMAID_VERSION=11.17.2
KATEX_VERSION=0.16.47
HLJS_VERSION=11.12.0

CDN=https://cdn.jsdelivr.net
MERMAID=$CDN/npm/mermaid@$MERMAID_VERSION/dist
KATEX=$CDN/npm/katex@$KATEX_VERSION/dist
HLJS=$CDN/gh/highlightjs/cdn-release@$HLJS_VERSION/build

# 出力先 | 加工 | URL
#   gzip      : gzip して保存（サーバーは Content-Encoding: gzip で返す）
#   katex-css : フォントの参照先を /lib/fonts/ に書き換えてから gzip
#   raw       : そのまま保存（woff2 は圧縮済み）
FILES=(
  "highlight.min.js.gz|gzip|$HLJS/highlight.min.js"
  "hljs-dark.min.css.gz|gzip|$HLJS/styles/github-dark.min.css"
  "hljs-light.min.css.gz|gzip|$HLJS/styles/github.min.css"
  "mermaid.min.js.gz|gzip|$MERMAID/mermaid.min.js"
  "katex.min.js.gz|gzip|$KATEX/katex.min.js"
  "auto-render.min.js.gz|gzip|$KATEX/contrib/auto-render.min.js"
  "katex.min.css.gz|katex-css|$KATEX/katex.min.css"
)
for font in \
  KaTeX_AMS-Regular KaTeX_Caligraphic-Bold KaTeX_Caligraphic-Regular \
  KaTeX_Fraktur-Bold KaTeX_Fraktur-Regular KaTeX_Main-Bold KaTeX_Main-BoldItalic \
  KaTeX_Main-Italic KaTeX_Main-Regular KaTeX_Math-BoldItalic KaTeX_Math-Italic \
  KaTeX_SansSerif-Bold KaTeX_SansSerif-Italic KaTeX_SansSerif-Regular \
  KaTeX_Script-Regular KaTeX_Size1-Regular KaTeX_Size2-Regular \
  KaTeX_Size3-Regular KaTeX_Size4-Regular KaTeX_Typewriter-Regular; do
  FILES+=("fonts/$font.woff2|raw|$KATEX/fonts/$font.woff2")
done

# ダウンロードした元のファイルの SHA-256（出力先ごと）
CHECKSUMS="
highlight.min.js.gz 8ab71eb09c51f501e5e25157d9cff100e46cc29bcbfc744d0b746d451fca7f53
hljs-dark.min.css.gz 9f208d022102b1d0c7aebfecd8e42ca7997d5de636649d2b31ea63093d809019
hljs-light.min.css.gz 3a9a5def8b9c311e5ae43abde85c63133185eed4f0d9f67fea4b00a8308cf066
mermaid.min.js.gz 581ed7d74bd9048d0e3a91363927d72ef22942d7722546b27f7cc29e35390eb8
katex.min.js.gz a29d2961d3146de5949d78ac7c1a9d93ae54955bad22a6db4fbe836e88e8bf48
auto-render.min.js.gz e5372d199bcdae8b4de71d0f7ceba72a4ba12774a27c60a6f1f77d03b3228ee4
katex.min.css.gz 0289a02cf451a44dd73add683a09644252363871ac11713a647b732cee8b1ee3
fonts/KaTeX_AMS-Regular.woff2 0cdd387c9590a1a9f9794560022dbb59654a7d86f187aa0c81495ad42d3a7308
fonts/KaTeX_Caligraphic-Bold.woff2 de7701e42cf1f4cf0b766c03fb27977207eee2f4fd5d76fa82188406da43ea4c
fonts/KaTeX_Caligraphic-Regular.woff2 5d53e70ad607c2352162dec9e0923fb54ecdafaccbf604cd8dcf7d00facb989b
fonts/KaTeX_Fraktur-Bold.woff2 74444efd593c005e3f4573b44524704c0af0a937fe911cca9e94068d0d140d3f
fonts/KaTeX_Fraktur-Regular.woff2 51814d270d06ff0255dba0799994fa4d8c84d11f09951d47595f4abb1f3602dc
fonts/KaTeX_Main-Bold.woff2 0f60d1b897938ec918c8ce073092411baf9438f6739465693ff18b0f9d20b021
fonts/KaTeX_Main-BoldItalic.woff2 99cd42a3c072d918f2f44984a807cf7aa16e13545fd0875fc07c6c65f99e715b
fonts/KaTeX_Main-Italic.woff2 97479ca6cce906abc961ecac96faa5f9ca2e61b8e7670d475826bcdee9a7c267
fonts/KaTeX_Main-Regular.woff2 c2342cd8b869e01752a9321dc17213fc40d4d04c79688c1d43f2cf316abd7866
fonts/KaTeX_Math-BoldItalic.woff2 dc47344dbb6cb5b655c8460d561f4df5f501b90c804ad3c6cec65fe322351ab1
fonts/KaTeX_Math-Italic.woff2 7af58c5ec8f132a2ddde9027c6d7814decce4d3b822a11192a42a20e2e973264
fonts/KaTeX_SansSerif-Bold.woff2 e99ae51144bf1232efcc1bfe5add36262c6866b0faab24fa75740e1b98577a62
fonts/KaTeX_SansSerif-Italic.woff2 00b26ac825e2095056396e0553b8ac26d3f8ad158c3826e28b4c45b385c4714a
fonts/KaTeX_SansSerif-Regular.woff2 68e8c73ef42afd3ccec58bf0fba302cce448938e7fc020a5e31f8a952eee1342
fonts/KaTeX_Script-Regular.woff2 036d4e95149b69ff9bcc0cd55771efeb25ffa3947293e69acd78d5ac328c684b
fonts/KaTeX_Size1-Regular.woff2 6b47c40166b6dbe21a5dfca7718413f2147fd2399be1ba605d8ad39cedf25dfe
fonts/KaTeX_Size2-Regular.woff2 d04c54219f9eaec6d4d4fd42dfb28785975a4794d6b2fc71e566b9cd6db842dd
fonts/KaTeX_Size3-Regular.woff2 73d591271b1604960cb10bb90fee021670af7297017e0e98480b332d11f51995
fonts/KaTeX_Size4-Regular.woff2 a4af7d414440a1c1790825cfb700cf9cf43b0f2c4b04f0ebc523011ad9853ec0
fonts/KaTeX_Typewriter-Regular.woff2 71d517d67827787cfabdf186914cc3358eda539e37931941f2b2fd4a21f68c0b
"

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/src/embedded_lib"
UPDATE=0
[ "${1:-}" = "--update-checksums" ] && UPDATE=1

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$DIR/fonts"

failed=0
for entry in "${FILES[@]}"; do
  IFS='|' read -r dest mode url <<<"$entry"
  expected="$(awk -v d="$dest" '$1 == d { print $2 }' <<<"$CHECKSUMS")"
  file="$tmp/download"
  curl -fsSL --retry 3 -o "$file" "$url"
  actual="$(sha256 "$file")"

  if [ "$UPDATE" = 1 ]; then
    echo "$dest $actual"
    continue
  fi
  if [ "$actual" != "$expected" ]; then
    echo "checksum mismatch: $dest" >&2
    echo "  url:      $url" >&2
    echo "  expected: $expected" >&2
    echo "  actual:   $actual" >&2
    failed=1
    continue
  fi

  case "$mode" in
    gzip)      gzip -9 -n -c "$file" >"$DIR/$dest" ;;
    katex-css) sed 's|url(fonts/|url(/lib/fonts/|g' "$file" | gzip -9 -n -c >"$DIR/$dest" ;;
    raw)       cp "$file" "$DIR/$dest" ;;
  esac
done

if [ "$UPDATE" = 1 ]; then
  exit 0
fi
if [ "$failed" = 1 ]; then
  echo "Some files did not match their checksums. Nothing was changed for them." >&2
  exit 1
fi
echo "Fetched ${#FILES[@]} files into $DIR"
