-- tests/smoke.lua
-- ヘッドレス Neovim で動くスモークテスト。リポジトリルートから実行する:
--   nvim --headless -u NONE -c "luafile tests/smoke.lua" -c "qa!"
-- 失敗時は cquit 1 で終了コード 1 を返す（CI で検出するため）

vim.opt.rtp:prepend(".")

local failures = {}
local passed = 0

--- 条件を検証する。失敗してもテストは続行し、最後にまとめて報告する
local function check(cond, msg)
  if cond then
    passed = passed + 1
  else
    failures[#failures + 1] = msg
  end
end

--- モジュールのロード
local ok, md = pcall(require, "md_preview")
check(ok, "require('md_preview') に失敗: " .. tostring(md))
if not ok then
  io.stderr:write("FAIL: " .. failures[1] .. "\n")
  vim.cmd("cquit 1")
  return
end

--- setup() がデフォルト値と深いマージをすること
md.setup({ port = 1234 })
check(md.config.port == 1234, "config.port が 1234 ではない: " .. tostring(md.config.port))
check(type(md.config.export) == "table", "config.export がテーブルではない")
check(md.config.export.pdf_engine == "browser",
  "config.export.pdf_engine が 'browser' ではない: " .. tostring(md.config.export.pdf_engine))
check(md.config.export.docx_engine == "pandoc",
  "config.export.docx_engine が 'pandoc' ではない: " .. tostring(md.config.export.docx_engine))
check(md.config.auto_open == true, "config.auto_open のデフォルトが true ではない")
check(type(md.config.binary) == "string" and md.config.binary ~= "", "config.binary が空")
check(md._initialized == true, "setup() 後に _initialized が true ではない")

--- ユーザーコマンドの登録
vim.cmd("runtime plugin/md_preview.lua")
local commands = vim.api.nvim_get_commands({})
for _, name in ipairs({
  "MarkdownPreview", "MarkdownPreviewStop", "MarkdownPreviewToggle",
  "MdPreview", "MdPreviewStop", "MdPreviewInstall",
}) do
  check(commands[name] ~= nil, "コマンド :" .. name .. " が登録されていない")
end

--- ヘルスチェックプロバイダ
local hok, health = pcall(require, "md_preview.health")
check(hok, "require('md_preview.health') に失敗: " .. tostring(health))
check(hok and type(health.check) == "function", "md_preview.health.check が関数ではない")

--- 初期状態ではプレビューは動いていない
check(type(md._is_running) == "function", "_is_running が関数ではない")
check(md._is_running() == false, "_is_running() が初期状態で false ではない")

--- ジョブの出力が途中で分かれて届いても、行として組み立てる（"READY:" と番号が別々に届く場合）
do
  local lines, rest = md._join_lines("", { "READY:" })
  check(#lines == 0 and rest == "READY:", "行の途中が残っていない")
  lines, rest = md._join_lines(rest, { "9950", "" })
  check(#lines == 1 and lines[1] == "READY:9950" and rest == "", "分かれた行がつながらない: " .. vim.inspect(lines))
  lines, rest = md._join_lines("", { "a", "b", "c" })
  check(#lines == 2 and lines[2] == "b" and rest == "c", "複数行の組み立てが違う: " .. vim.inspect(lines))
end

--- 結果
if #failures > 0 then
  for _, msg in ipairs(failures) do
    io.stderr:write("FAIL: " .. msg .. "\n")
  end
  io.stderr:write(string.format("%d passed, %d failed\n", passed, #failures))
  vim.cmd("cquit 1")
  return
end

print(string.format("PASS: %d checks (md_preview smoke test)", passed))
