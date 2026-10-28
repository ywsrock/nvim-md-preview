-- lua/md_preview/health.lua
-- :checkhealth md_preview 用のヘルスチェック
-- バイナリ・エクスポートエンジン・ダウンロード手段・ブラウザ起動手段を確認する

local M = {}

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
    return configured, "configured"
  end
  local auto = vim.fn.stdpath("data") .. "/nvim-md-preview/" .. name .. exe
  if vim.fn.executable(auto) == 1 then
    return auto, "downloaded"
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
    h.warn(string.format("Neovim %s is too old", ver),
      { "Update to Neovim 0.9 or later" })
  end
end

--- プレビューバイナリ
local function check_binary(config)
  h.start("Preview binary")
  local bin = config.binary
  if vim.fn.executable(bin) == 1 then
    h.ok(string.format("Binary: %s", bin))
    local md = require("md_preview")
    local v = md._binary_version(bin)
    if v == md.version then
      h.ok(string.format("Version: v%s (matches the plugin)", v))
    else
      h.warn(string.format("Version: %s, but the plugin is v%s", v and ("v" .. v) or "older than v0.5.0", md.version), {
        "Run :MdPreviewInstall to download the binary for this plugin version",
      })
    end
  elseif vim.fn.filereadable(bin) == 1 then
    h.error(string.format("The binary is not executable: %s", bin), {
      "chmod +x " .. bin,
      "Or download it again with :MdPreviewInstall",
    })
  else
    h.error(string.format("Binary not found: %s", bin), {
      "Run :MdPreviewInstall",
      'Or set the path with setup({ binary = "<path>" })',
    })
  end
end

--- エクスポートエンジン（pandoc / typst）
local function check_engine(name, configured, formats)
  local path, source = resolve_engine(name, configured)
  if not path then
    h.warn(string.format("%s not found (%s export is unavailable)", name, formats), {
      "Run :MdPreviewInstall",
      string.format('Or set the path with setup({ export = { %s_path = "<path>" } })', name),
    })
    return
  end
  if vim.fn.executable(path) ~= 1 then
    h.warn(string.format("%s is not executable: %s (%s export is unavailable)", name, path, formats), {
      string.format("Check the path in setup({ export = { %s_path = ... } })", name),
    })
    return
  end
  local ver = first_line({ path, "--version" })
  h.ok(string.format("%s: %s (%s)", name, path, source))
  if ver then
    h.info(ver)
  else
    h.warn(string.format("%s --version failed", name))
  end
end

local function check_export(config)
  h.start("Export engines")
  local export = config.export or {}
  check_engine("pandoc", export.pandoc_path, "DOCX / HTML / EPUB")
  -- PDF はブラウザで印刷するのが既定。ブラウザが無いときは Typst を使う
  local md = require("md_preview")
  if export.pdf_engine == "browser" then
    local browser = md._find_browser()
    if browser then
      h.ok(string.format("PDF: printed with %s", browser))
    else
      h.warn("PDF: no Chrome, Edge, Brave or Chromium found, so Typst is used (Mermaid, math and callouts are not rendered)", {
        "Install one of those browsers, or set export.browser_path",
      })
      check_engine("typst", export.typst_path, "PDF")
    end
  else
    check_engine("typst", export.typst_path, "PDF")
  end
end

--- ダウンロード手段（:MdPreviewInstall で使う）
local function check_download_tools()
  h.start("Download tools")
  if vim.fn.executable("curl") == 1 then
    h.ok("curl: " .. vim.fn.exepath("curl"))
  elseif vim.fn.executable("wget") == 1 then
    h.ok("wget: " .. vim.fn.exepath("wget"))
  elseif is_win and vim.fn.executable("powershell") == 1 then
    h.ok("powershell: " .. vim.fn.exepath("powershell"))
  else
    h.warn("None of curl, wget or PowerShell found (:MdPreviewInstall will not work)", {
      "Install curl or wget",
    })
  end

  if vim.fn.executable("tar") == 1 then
    h.ok("tar: " .. vim.fn.exepath("tar"))
  else
    h.info("tar not found (uncompressed files will be downloaded instead of .tar.gz)")
  end
end

--- ブラウザ起動手段（M._open_browser と同じ優先順位）
local function check_browser_opener(config)
  h.start("Browser")
  if vim.fn.has("mac") == 1 then
    if vim.fn.executable("open") == 1 then
      h.ok("open (macOS)")
    else
      h.warn("The open command was not found")
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
      h.warn("No command to open the browser was found", {
        "Install the wslu package (wslview)",
      })
    end
  elseif vim.fn.has("unix") == 1 then
    if vim.fn.executable("xdg-open") == 1 then
      h.ok("xdg-open (Linux)")
    else
      h.warn("xdg-open not found", {
        "Install xdg-utils",
      })
    end
  else
    if vim.fn.executable("cmd.exe") == 1 then
      h.ok("cmd.exe (Windows)")
    else
      h.warn("cmd.exe not found")
    end
  end
  h.info("auto_open = " .. tostring(config.auto_open))
end

--- ポート設定
local function check_port(config)
  h.start("Server")
  h.info(string.format("port = %s (the next free port is used if it is taken)", tostring(config.port)))
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
