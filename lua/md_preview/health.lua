-- lua/md_preview/health.lua
-- :checkhealth md_preview 用のヘルスチェック
-- バイナリ・エクスポートエンジン・ダウンロード手段・ブラウザ起動手段を確認する

local M = {}

local tr = require("md_preview.i18n").tr

-- vim.health の API（Neovim 0.9 以前は report_* 系の名前）
local health = vim.health or {}
local h = {
  start = health.start or health.report_start,
  ok    = health.ok    or health.report_ok,
  warn  = health.warn  or health.report_warn,
  error = health.error or health.report_error,
  info  = health.info  or health.report_info,
}

local is_win = vim.fn.has("win32") == 1
local exe    = is_win and ".exe" or ""

--- コマンドを実行して標準出力の 1 行目を返す（失敗時は nil）
local function first_line(cmd)
  local ok, out = pcall(vim.fn.systemlist, cmd)
  if not ok or vim.v.shell_error ~= 0 or type(out) ~= "table" or #out == 0 then
    return nil
  end
  return vim.trim(out[1])
end

--- WSL 環境かどうかを検出する（lua/md_preview.lua と同じ判定）
local function detect_wsl()
  local f = io.open("/proc/version", "r")
  if not f then return false end
  local content = f:read("*a"):lower()
  f:close()
  return content:find("microsoft") ~= nil or content:find("wsl") ~= nil
end

--- エクスポートエンジンのパスを解決する（lua/md_preview.lua の send_export_config と同じ順序）
---   1. config.export.<name>_path
---   2. stdpath("data")/nvim-md-preview/<name>[.exe]
---   3. PATH 上の <name>
--- 戻り値: (path, source) / 見つからなければ nil
local function resolve_engine(name, configured)
  if configured ~= nil and configured ~= "" then
    return configured, tr("configured", "設定値")
  end
  local auto = vim.fn.stdpath("data") .. "/nvim-md-preview/" .. name .. exe
  if vim.fn.executable(auto) == 1 then
    return auto, tr("downloaded", "自動ダウンロード先")
  end
  if vim.fn.executable(name) == 1 then
    return vim.fn.exepath(name), "PATH"
  end
  return nil
end

--- Neovim バージョン
local function check_nvim()
  h.start("Neovim")
  local v = vim.version()
  local ver = string.format("%d.%d.%d", v.major, v.minor, v.patch)
  if v.major > 0 or v.minor >= 9 then
    h.ok("Neovim " .. ver)
  else
    h.warn(tr("Neovim %s is too old", "Neovim %s は古すぎます", ver),
      { tr("Update to Neovim 0.9 or later", "Neovim 0.9 以上にアップデートしてください") })
  end
end

--- プレビューバイナリ
local function check_binary(config)
  h.start(tr("Preview binary", "プレビューバイナリ"))
  local bin = config.binary
  if vim.fn.executable(bin) == 1 then
    h.ok(tr("Binary: %s", "バイナリ: %s", bin))
  elseif vim.fn.filereadable(bin) == 1 then
    h.error(tr("The binary is not executable: %s", "バイナリに実行権限がありません: %s", bin), {
      "chmod +x " .. bin,
      tr("Or download it again with :MdPreviewInstall", "または :MdPreviewInstall で再ダウンロードしてください"),
    })
  else
    h.error(tr("Binary not found: %s", "バイナリが見つかりません: %s", bin), {
      tr("Run :MdPreviewInstall", ":MdPreviewInstall を実行してください"),
      tr('Or set the path with setup({ binary = "<path>" })', 'または setup({ binary = "<path>" }) でパスを指定してください'),
    })
  end
end

--- エクスポートエンジン（pandoc / typst）
local function check_engine(name, configured, formats)
  local path, source = resolve_engine(name, configured)
  if not path then
    h.warn(tr("%s not found (%s export is unavailable)", "%s が見つかりません（%s エクスポートは利用できません）", name, formats), {
      tr("Run :MdPreviewInstall", ":MdPreviewInstall を実行してください"),
      tr('Or set the path with setup({ export = { %s_path = "<path>" } })',
        'または setup({ export = { %s_path = "<path>" } }) でパスを指定してください', name),
    })
    return
  end
  if vim.fn.executable(path) ~= 1 then
    h.warn(tr("%s is not executable: %s (%s export is unavailable)", "%s が実行できません: %s（%s エクスポートは利用できません）",
      name, path, formats), {
      tr("Check the path in setup({ export = { %s_path = ... } })", "setup({ export = { %s_path = ... } }) のパスを確認してください", name),
    })
    return
  end
  local ver = first_line({ path, "--version" })
  h.ok(string.format("%s: %s (%s)", name, path, source))
  if ver then
    h.info(ver)
  else
    h.warn(tr("%s --version failed", "%s --version の実行に失敗しました", name))
  end
end

local function check_export(config)
  h.start(tr("Export engines", "エクスポートエンジン"))
  local export = config.export or {}
  check_engine("pandoc", export.pandoc_path, "DOCX / HTML / EPUB")
  check_engine("typst", export.typst_path, "PDF")
end

--- ダウンロード手段（:MdPreviewInstall で使う）
local function check_download_tools()
  h.start(tr("Download tools", "ダウンロードツール"))
  if vim.fn.executable("curl") == 1 then
    h.ok("curl: " .. vim.fn.exepath("curl"))
  elseif vim.fn.executable("wget") == 1 then
    h.ok("wget: " .. vim.fn.exepath("wget"))
  elseif is_win and vim.fn.executable("powershell") == 1 then
    h.ok("powershell: " .. vim.fn.exepath("powershell"))
  else
    h.warn(tr("None of curl, wget or PowerShell found (:MdPreviewInstall will not work)",
      "curl / wget / PowerShell のいずれも見つかりません（:MdPreviewInstall が使えません）"), {
      tr("Install curl or wget", "curl または wget をインストールしてください"),
    })
  end

  if vim.fn.executable("tar") == 1 then
    h.ok("tar: " .. vim.fn.exepath("tar"))
  else
    h.info(tr("tar not found (uncompressed files will be downloaded instead of .tar.gz)",
      "tar が見つかりません（.tar.gz の代わりに非圧縮版をダウンロードします）"))
  end
end

--- ブラウザ起動手段（M._open_browser と同じ優先順位）
local function check_browser_opener(config)
  h.start(tr("Browser", "ブラウザ"))
  if vim.fn.has("mac") == 1 then
    if vim.fn.executable("open") == 1 then
      h.ok("open (macOS)")
    else
      h.warn(tr("The open command was not found", "open コマンドが見つかりません"))
    end
  elseif detect_wsl() then
    if vim.fn.executable("wslview") == 1 then
      h.ok("wslview (WSL)")
    elseif vim.fn.executable("powershell.exe") == 1 then
      h.ok("powershell.exe (WSL)")
    elseif vim.fn.executable("cmd.exe") == 1 then
      h.ok("cmd.exe (WSL)")
    elseif vim.fn.executable("xdg-open") == 1 then
      h.ok("xdg-open (WSL)")
    else
      h.warn(tr("No command to open the browser was found", "ブラウザを開くコマンドが見つかりません"), {
        tr("Install the wslu package (wslview)", "wslu パッケージ（wslview）をインストールしてください"),
      })
    end
  elseif vim.fn.has("unix") == 1 then
    if vim.fn.executable("xdg-open") == 1 then
      h.ok("xdg-open (Linux)")
    else
      h.warn(tr("xdg-open not found", "xdg-open が見つかりません"), {
        tr("Install xdg-utils", "xdg-utils をインストールしてください"),
      })
    end
  else
    if vim.fn.executable("cmd.exe") == 1 then
      h.ok("cmd.exe (Windows)")
    else
      h.warn(tr("cmd.exe not found", "cmd.exe が見つかりません"))
    end
  end
  h.info("auto_open = " .. tostring(config.auto_open))
end

--- ポート設定
local function check_port(config)
  h.start(tr("Server", "サーバー"))
  h.info(tr("port = %s (the next free port is used if it is taken)", "port = %s（使用中の場合は自動で空きポートを使用します）",
    tostring(config.port)))
end

function M.check()
  local ok, md = pcall(require, "md_preview")
  if not ok then
    h.start("md_preview")
    h.error('require("md_preview") failed: ' .. tostring(md))
    return
  end
  local config = md.config or {}

  check_nvim()
  check_binary(config)
  check_export(config)
  check_download_tools()
  check_browser_opener(config)
  check_port(config)
end

return M
