-- lua/md_preview/i18n.lua
-- メッセージの言語（英語 / 日本語）を切り替える

local M = {}

-- setup() の lang（"auto" | "en" | "ja"）。md_preview.setup から設定される
M.setting = "auto"

--- 実際に使う言語。"auto" は Neovim / 環境変数の言語設定が日本語なら "ja"、それ以外は "en"
function M.lang()
  if M.setting == "en" or M.setting == "ja" then return M.setting end
  local candidates = { vim.v.lang, os.getenv("LC_ALL"), os.getenv("LC_MESSAGES"), os.getenv("LANG") }
  for _, l in ipairs(candidates) do
    if l and l ~= "" and l ~= "C" and l ~= "POSIX" then
      return l:lower():match("^ja") and "ja" or "en"
    end
  end
  return "en"
end

--- 言語に合った文を返す。en / ja は string.format の書式で、残りの引数で埋める
function M.tr(en, ja, ...)
  local s = M.lang() == "ja" and ja or en
  if select("#", ...) > 0 then return string.format(s, ...) end
  return s
end

return M
