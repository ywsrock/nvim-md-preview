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

--- browser_app: --app で開くコマンド。無効なら今までどおり、ブラウザが無ければ通常の開き方に戻る
do
  local url = "http://127.0.0.1:9876/?instance=x&token=y"
  local default_cmd = md._browser_cmd(url)
  check(not vim.tbl_contains(default_cmd, "--app=" .. url), "browser_app = false で --app が付いている")
  md.config.browser_app = true
  md.config.export.browser_path = vim.v.progpath  -- 実行できるファイルならブラウザの代わりになる
  local app = md._browser_cmd(url)
  check(app[1] == vim.v.progpath and app[2] == "--app=" .. url and #app == 2,
    "browser_app のコマンドが違う: " .. vim.inspect(app))
  md.config.export.browser_path = "/nonexistent/browser"
  check(vim.deep_equal(md._browser_cmd(url), default_cmd), "ブラウザが無いときに通常の開き方に戻らない")
  md.config.browser_app = false
  md.config.export.browser_path = ""
end

--- pandoc / Typst: 設定したパスや PATH にあればダウンロードしない（#44）
do
  local is_win = vim.fn.has("win32") == 1
  local data = vim.fn.tempname()
  vim.fn.mkdir(data, "p")
  local orig_stdpath = vim.fn.stdpath
  vim.fn.stdpath = function(what)  -- ダウンロード先を空の一時ディレクトリにする
    if what == "data" then return data end
    return orig_stdpath(what)
  end
  local bin = vim.fn.tempname()
  vim.fn.mkdir(bin, "p")
  local orig_path = vim.env.PATH
  local function on_path(name)
    local f = bin .. "/" .. name .. (is_win and ".bat" or "")
    vim.fn.writefile(is_win and { "@echo off" } or { "#!/bin/sh" }, f)
    if not is_win then vim.fn.setfperm(f, "rwxr-xr-x") end
  end
  local function find(items, label)
    for _, it in ipairs(items) do
      if it.label == label then return it end
    end
  end
  local ex = md.config.export
  ex.pdf_engine = "typst"

  -- PATH に無ければダウンロードする
  vim.env.PATH = bin
  check(md._find_engine("pandoc", "") == nil, "PATH に無い pandoc が見つかった")
  local items = md._install_items({})
  check(find(items, "pandoc") and find(items, "pandoc").status == nil, "PATH に無い pandoc をダウンロードしない")

  -- PATH にあればダウンロードしない。Typst のパッケージは入れる
  on_path("pandoc")
  on_path("typst")
  local path, source = md._find_engine("pandoc", "")
  check(source == "PATH" and path ~= "", "PATH 上の pandoc が見つからない: " .. tostring(source))
  items = md._install_items({})
  check(find(items, "pandoc").status == "skip" and find(items, "pandoc").found == path,
    "PATH に pandoc があるのにダウンロードする: " .. vim.inspect(find(items, "pandoc")))
  check(find(items, "typst").status == "skip", "PATH に typst があるのにダウンロードする")
  check(find(items, "pkgs") and find(items, "pkgs").status == nil, "Typst のパッケージを入れない")

  -- force（:MdPreviewInstall!）ならダウンロードする
  items = md._install_items({ force = true })
  check(find(items, "pandoc").status == nil, "force でも pandoc をダウンロードしない")

  -- 設定したパスが最優先
  ex.pandoc_path = "/opt/pandoc"
  path, source = md._find_engine("pandoc", ex.pandoc_path)
  check(source == "configured" and path == "/opt/pandoc", "設定したパスが優先されない")
  check(find(md._install_items({}), "pandoc").status == "skip", "pandoc_path を設定してもダウンロードする")
  ex.pandoc_path = ""

  -- ダウンロードしたものがあれば、PATH より優先し、今までどおり更新する
  local dl = data .. "/nvim-md-preview"
  vim.fn.mkdir(dl, "p")
  local auto = dl .. "/pandoc" .. (is_win and ".exe" or "")
  vim.fn.writefile({ "#!/bin/sh" }, auto)
  if not is_win then vim.fn.setfperm(auto, "rwxr-xr-x") end
  if vim.fn.executable(auto) == 1 then
    _, source = md._find_engine("pandoc", "")
    check(source == "downloaded", "ダウンロードしたものが優先されない: " .. tostring(source))
    check(find(md._install_items({}), "pandoc").status == nil, "ダウンロードしたものを更新しない")
    check(find(md._install_items({ missing_only = true }), "pandoc").status == "done",
      "missing_only でダウンロード済みのものをスキップしない")
  end

  vim.env.PATH = orig_path
  vim.fn.stdpath = orig_stdpath
  ex.pdf_engine = "browser"
  vim.fn.delete(data, "rf")
  vim.fn.delete(bin, "rf")
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
