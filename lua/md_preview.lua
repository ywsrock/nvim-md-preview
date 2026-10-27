-- lua/md_preview.lua
-- Neovim 側のグルーコード
-- Rust バイナリを起動し、バッファ変更を RPC 経由で送る

local M = {}

M._initialized = false

local _plugin_dir = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":h:h")

-- 設定のデフォルト値
M.config = {
  binary = vim.fn.stdpath("data") .. "/nvim-md-preview/nvim-md-preview"
    .. (vim.fn.has("win32") == 1 and ".exe" or ""),
  port = 9876,
  auto_open = true,

  -- エクスポート設定
  export = {
    -- PDF エンジン: "typst"（高品質・推奨）| "print"（ブラウザ印刷・依存なし）
    pdf_engine  = "typst",
    -- DOCX/HTML/EPUB エンジン: "pandoc"（推奨）| "none"（無効）
    docx_engine = "pandoc",
    -- 表示するエクスポート形式（不要なものは削除可）
    formats = { "pdf", "docx", "html", "epub", "markdown" },
    -- バイナリパス（空の場合は自動ダウンロード先 ~/.local/share/nvim/nvim-md-preview/ を使用）
    pandoc_path = "",
    typst_path  = "",
  },
}

local state = {
  job_id      = nil,  -- Rust バイナリのジョブID
  chan_id      = nil,  -- RPC チャネルID
  buf_id       = nil,  -- 監視中のバッファ ID
  actual_port  = nil,  -- サーバーが実際に使用しているポート（ポート競合時に変わる）
}

--- プレビューが実行中かどうか
function M._is_running()
  return state.job_id ~= nil
end

--- バッファ全文を文字列として取得する
local function get_buf_content(buf)
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  return table.concat(lines, "\n")
end

--- バッファのファイル名を取得する（未保存なら "md Preview"）
local function get_buf_title(buf)
  local name = vim.api.nvim_buf_get_name(buf)
  local filename = vim.fn.fnamemodify(name, ":t")
  if filename == "" then return "md Preview" end
  return filename
end

--- バッファのディレクトリを取得する（相対パス画像の解決に使う）
local function get_buf_dir(buf)
  local name = vim.api.nvim_buf_get_name(buf)
  if name == "" then return "" end
  return vim.fn.fnamemodify(name, ":h")
end

--- Rust バイナリに Markdown テキストを送る
local function send_update(buf)
  if state.chan_id == nil then return end
  local content   = get_buf_content(buf)
  local title     = get_buf_title(buf)
  local base_dir  = get_buf_dir(buf)
  local file_path = vim.api.nvim_buf_get_name(buf)  -- フルパス（エクスポート用）
  pcall(vim.rpcnotify, state.chan_id, "md_preview_update", content, title, base_dir, file_path)
end

--- Rust バイナリにカーソル行を送る（スクロール同期）
local function send_scroll(buf)
  if state.chan_id == nil then return end
  local line  = vim.api.nvim_win_get_cursor(0)[1]
  local total = vim.api.nvim_buf_line_count(buf)
  pcall(vim.rpcnotify, state.chan_id, "md_preview_scroll",
    math.floor(line), math.floor(total))
end

--- エクスポートエンジンのパスを解決してバイナリに送る
local function send_export_config()
  if state.chan_id == nil then return end

  local install_dir = vim.fn.stdpath("data") .. "/nvim-md-preview"
  local is_win = vim.fn.has("win32") == 1

  local pandoc_path = M.config.export.pandoc_path
  if pandoc_path == "" then
    local auto = install_dir .. "/pandoc" .. (is_win and ".exe" or "")
    if vim.fn.executable(auto) == 1 then
      pandoc_path = auto
    elseif vim.fn.executable("pandoc") == 1 then
      pandoc_path = "pandoc"
    end
  end

  local typst_path = M.config.export.typst_path
  if typst_path == "" then
    local auto = install_dir .. "/typst" .. (is_win and ".exe" or "")
    if vim.fn.executable(auto) == 1 then
      typst_path = auto
    elseif vim.fn.executable("typst") == 1 then
      typst_path = "typst"
    end
  end

  pcall(vim.rpcnotify, state.chan_id, "md_preview_config", pandoc_path, typst_path)
end

--- バッファの変更イベントを購読する
local function attach_buffer(buf)
  if state.buf_id == buf then return end

  state.buf_id = buf

  -- テキスト変更イベント
  vim.api.nvim_buf_attach(buf, false, {
    on_lines = function(_, changed_buf, _, _, _, _)
      vim.schedule(function()
        send_update(changed_buf)
      end)
    end,
    on_detach = function()
      state.buf_id = nil
    end,
  })

  -- カーソル移動でスクロール同期（バッファごとに1つだけ登録）
  vim.api.nvim_create_autocmd({ "CursorMoved", "CursorMovedI" }, {
    buffer = buf,
    group  = vim.api.nvim_create_augroup("md_preview_scroll_" .. buf, { clear = true }),
    callback = function()
      -- vim.schedule で非同期実行（chan_id が設定される前に呼ばれても安全）
      vim.schedule(function()
        if state.chan_id ~= nil and state.buf_id == buf then
          send_scroll(buf)
        end
      end)
    end,
  })

  -- 初回の内容を即座に送る
  send_update(buf)
end

--- Rust バイナリを起動する
local function start_binary()
  if state.job_id ~= nil then
    return true  -- 既に起動済み
  end

  -- stdin/stdout RPC 方式のため --listen は不要
  local cmd = {
    M.config.binary,
    tostring(M.config.port),
  }

  -- Rust バイナリを RPC ジョブとして起動
  state.job_id = vim.fn.jobstart(cmd, {
    rpc = true,
    on_exit = function(_, exit_code, _)
      vim.notify(
        string.format("[md-preview] バイナリが終了しました (exit: %d)", exit_code),
        vim.log.levels.WARN
      )
      state.job_id = nil
      state.chan_id = nil
      -- バッファに登録したオートコマンド（スクロール同期）を削除
      if state.buf_id ~= nil then
        local ok, _ = pcall(vim.api.nvim_del_augroup_by_name, "md_preview_scroll_" .. state.buf_id)
        if not ok then end  -- 存在しない場合は無視
        state.buf_id = nil
      end
    end,
    on_stderr = function(_, data, _)
      for _, line in ipairs(data) do
        if line ~= "" then
          -- READY:<port> でサーバーが使用しているポートを受け取る
          local port_str = line:match("^READY:(%d+)$")
          if port_str then
            state.actual_port = tonumber(port_str)
          end
        end
      end
    end,
  })

  if state.job_id <= 0 then
    vim.notify("[md-preview] バイナリの起動に失敗しました: " .. M.config.binary, vim.log.levels.ERROR)
    state.job_id = nil
    return false
  end

  state.chan_id = state.job_id
  return true
end

--- プレビューを開始する（:MdPreview コマンドから呼び出す）
function M.start()
  local buf = vim.api.nvim_get_current_buf()
  local filetype = vim.bo[buf].filetype

  if filetype ~= "markdown" then
    vim.notify("[md-preview] Markdown ファイルではありません (filetype: " .. filetype .. ")", vim.log.levels.WARN)
    return
  end

  -- バイナリが存在しなければ自動ダウンロードを試みる
  if vim.fn.executable(M.config.binary) == 0 then
    vim.notify("[md-preview] バイナリが見つかりません。ダウンロードを試みます...", vim.log.levels.INFO)
    if not M.install() then
      vim.notify(
        "[md-preview] 自動ダウンロード失敗。手動でインストールしてください:\n:MdPreviewInstall",
        vim.log.levels.ERROR
      )
      return
    end
  end

  if not start_binary() then return end

  -- バッファを監視開始
  attach_buffer(buf)
  vim.defer_fn(function()
    send_update(buf)
  end, 500)

  -- エンジンパスをバイナリに送る（500ms 後、バイナリ起動完了後）
  vim.defer_fn(function()
    send_export_config()
  end, 600)

  -- READY:<port> が届くまで少し待ってからブラウザを開く
  vim.defer_fn(function()
    local port = state.actual_port or M.config.port
    local url = string.format("http://127.0.0.1:%d", port)
    if M.config.auto_open then
      M._open_browser(url)
    end
    vim.notify("[md-preview] プレビュー開始: " .. url, vim.log.levels.INFO)
  end, 300)
end

--- プレビューを停止する（:MdPreviewStop コマンドから呼び出す）
function M.stop()
  if state.job_id ~= nil then
    vim.fn.jobstop(state.job_id)
    state.job_id = nil
    state.chan_id = nil
  end
  if state.buf_id ~= nil then
    -- スクロール同期オートコマンドを削除
    pcall(vim.api.nvim_del_augroup_by_name, "md_preview_scroll_" .. state.buf_id)
    state.buf_id = nil
  end
  vim.notify("[md-preview] プレビュー停止", vim.log.levels.INFO)
end

--- WSL 環境かどうかを検出する
local function detect_wsl()
  -- /proc/version に "microsoft" / "WSL" が含まれていれば WSL
  local f = io.open("/proc/version", "r")
  if not f then return false end
  local content = f:read("*a"):lower()
  f:close()
  return content:find("microsoft") ~= nil or content:find("wsl") ~= nil
end

--- OS・環境に応じてブラウザを開く
--- WSL の場合は Windows 側のブラウザを優先する
function M._open_browser(url)
  local cmd

  if vim.fn.has("mac") == 1 then
    -- macOS
    cmd = { "open", url }

  elseif detect_wsl() then
    -- WSL: Windows 側のブラウザを優先
    --   1. wslview (wslu パッケージ) — 最も確実
    --   2. powershell.exe — WSL2 どこでも使える
    --   3. cmd.exe /c start — フォールバック
    --   4. xdg-open — WSL 内にブラウザがある場合の最終手段
    if vim.fn.executable("wslview") == 1 then
      cmd = { "wslview", url }
    elseif vim.fn.executable("powershell.exe") == 1 then
      -- PowerShell の Start-Process でデフォルトブラウザを開く
      cmd = { "powershell.exe", "-NoProfile", "-Command",
              string.format("Start-Process '%s'", url) }
    elseif vim.fn.executable("cmd.exe") == 1 then
      -- cmd.exe /c start "title" url  （空タイトルが必要）
      cmd = { "cmd.exe", "/c", "start", '""', url }
    else
      cmd = { "xdg-open", url }
    end

  elseif vim.fn.has("unix") == 1 then
    -- 通常の Linux
    cmd = { "xdg-open", url }

  else
    -- Windows (WSL でない)
    cmd = { "cmd.exe", "/c", "start", '""', url }
  end

  vim.fn.jobstart(cmd, { detach = true })
end

--- GitHub Releases から OS に合わせたバイナリをダウンロードする
--- lazy.nvim の build オプションや :MdPreviewInstall コマンドから呼ぶ
function M.install()
  local uname    = vim.loop.os_uname()
  local sysname  = uname.sysname  -- "Linux" / "Darwin" / "Windows_NT"
  local machine  = uname.machine  -- "x86_64" / "arm64" / "aarch64"

  local is_win = sysname:match("Windows") ~= nil or vim.fn.has("win32") == 1
  local ext    = is_win and ".exe" or ""
  local install_dir = vim.fn.stdpath("data") .. "/nvim-md-preview"
  vim.fn.mkdir(install_dir, "p")

  -- ── 1. nvim-md-preview バイナリ ──────────────────────────────
  local target
  if sysname == "Linux" then
    target = "x86_64-unknown-linux-gnu"
  elseif sysname == "Darwin" then
    target = (machine == "arm64" or machine == "aarch64")
      and "aarch64-apple-darwin" or "x86_64-apple-darwin"
  else
    target = "x86_64-pc-windows-msvc"
  end

  local fname  = "nvim-md-preview-" .. target .. ext
  local url    = string.format("https://github.com/ywsrock/nvim-md-preview/releases/latest/download/%s", fname)
  local binary = install_dir .. "/nvim-md-preview" .. ext

  local ok = M._download(url, binary)
  if not ok then
    vim.notify(
      "[md-preview] バイナリのダウンロードに失敗しました。\n"
      .. "手動でダウンロードして配置してください:\n"
      .. "  URL: " .. url .. "\n"
      .. "  保存先: " .. binary,
      vim.log.levels.ERROR
    )
    return false
  else
    if not is_win then vim.fn.system('chmod +x "' .. binary .. '"') end
    M.config.binary = binary
    vim.notify("[md-preview] ✅ nvim-md-preview: " .. binary, vim.log.levels.INFO)
  end

  -- ── 2. pandoc（DOCX/HTML/EPUB 用） ───────────────────────────
  local cfg = M.config.export
  if cfg.docx_engine == "pandoc" then
    vim.notify("[md-preview] pandoc をダウンロード中...", vim.log.levels.INFO)
    local pandoc_ok = M._install_pandoc(install_dir, sysname, machine, is_win, ext)
    if pandoc_ok then
      vim.notify("[md-preview] ✅ pandoc インストール完了", vim.log.levels.INFO)
    else
      vim.notify("[md-preview] ⚠️  pandoc のダウンロードに失敗しました。手動でインストールしてください。", vim.log.levels.WARN)
    end
  end

  -- ── 3. Typst（PDF 用） ────────────────────────────────────────
  if cfg.pdf_engine == "typst" then
    vim.notify("[md-preview] Typst をダウンロード中...", vim.log.levels.INFO)
    local typst_ok = M._install_typst(install_dir, sysname, machine, is_win, ext)
    if typst_ok then
      vim.notify("[md-preview] ✅ Typst インストール完了", vim.log.levels.INFO)
    else
      vim.notify("[md-preview] ⚠️  Typst のダウンロードに失敗しました。PDF はブラウザ印刷を使用します。", vim.log.levels.WARN)
    end
  end

  vim.notify("[md-preview] インストール完了！", vim.log.levels.INFO)
  return true
end

--- ファイルをダウンロードするヘルパー
function M._download(url, dest)
  local cmd
  if vim.fn.executable("curl") == 1 then
    -- -L: リダイレクト追跡, -s: 非表示, --fail-with-body: エラー時失敗
    -- Windows の curl も対応（-f の代わりに --fail-with-body または -s -S を使用）
    cmd = string.format('curl -L -s -S -o "%s" "%s"', dest, url)
  elseif vim.fn.executable("wget") == 1 then
    cmd = string.format('wget -q -O "%s" "%s"', dest, url)
  elseif vim.fn.has("win32") == 1 then
    -- PowerShell でのダウンロード（curl も wget もない場合のフォールバック）
    cmd = string.format(
      'powershell -NoProfile -Command "Invoke-WebRequest -Uri \'%s\' -OutFile \'%s\' -UseBasicParsing"',
      url, dest
    )
  else
    vim.notify("[md-preview] curl または wget が必要です", vim.log.levels.ERROR)
    return false
  end
  vim.fn.system(cmd)
  -- ダウンロードされたファイルが存在し、0 バイトでないことを確認
  return vim.v.shell_error == 0 and vim.fn.getfsize(dest) > 0
end

--- pandoc をインストール
function M._install_pandoc(install_dir, sysname, machine, is_win, ext)
  -- pandoc はバージョン番号入りファイル名なので GitHub API でバージョンを取得
  local version_cmd
  if vim.fn.executable("curl") == 1 then
    version_cmd = 'curl -sL "https://api.github.com/repos/jgm/pandoc/releases/latest"'
  elseif vim.fn.has("win32") == 1 then
    version_cmd = 'powershell -NoProfile -Command "(Invoke-WebRequest -Uri \'https://api.github.com/repos/jgm/pandoc/releases/latest\' -UseBasicParsing).Content"'
  else
    return false
  end

  local json_raw = vim.fn.system(version_cmd)
  local version = json_raw:match('"tag_name"%s*:%s*"([^"]+)"')
  if not version then
    vim.notify("[md-preview] pandoc のバージョン取得に失敗しました", vim.log.levels.WARN)
    return false
  end

  -- ファイル名決定（例: pandoc-3.13-windows-x86_64.zip）
  local fname
  if sysname == "Linux" then
    fname = string.format("pandoc-%s-linux-amd64.tar.gz", version)
  elseif sysname == "Darwin" then
    fname = (machine == "arm64" or machine == "aarch64")
      and string.format("pandoc-%s-arm64-macOS.zip", version)
       or string.format("pandoc-%s-x86_64-macOS.zip", version)
  else
    fname = string.format("pandoc-%s-windows-x86_64.zip", version)
  end

  local url     = string.format("https://github.com/jgm/pandoc/releases/download/%s/%s", version, fname)
  local archive = install_dir .. "/" .. fname
  local pandoc  = install_dir .. "/pandoc" .. ext

  vim.notify("[md-preview] pandoc " .. version .. " をダウンロード中...", vim.log.levels.INFO)
  if not M._download(url, archive) then return false end

  -- 解凍
  if fname:match("%.tar%.gz$") then
    -- Linux: tar.gz から bin/pandoc を取り出す
    vim.fn.system(string.format('tar -xzf "%s" -C "%s" --wildcards "*/bin/pandoc" --strip-components=2', archive, install_dir))
    -- 解凍後パスを確認して移動
    local found = install_dir .. "/bin/pandoc"
    if vim.fn.filereadable(found) == 1 then
      vim.fn.system(string.format('mv "%s" "%s"', found, pandoc))
      vim.fn.system('rmdir "' .. install_dir .. '/bin" 2>/dev/null; true')
    end
  elseif is_win then
    -- Windows: PowerShell で解凍
    vim.fn.system(string.format(
      'powershell -NoProfile -Command "Expand-Archive -Path \'%s\' -DestinationPath \'%s\' -Force"',
      archive, install_dir .. "/pandoc_tmp"
    ))
    -- pandoc.exe を探して移動
    local found = vim.fn.glob(install_dir .. "/pandoc_tmp/**/pandoc.exe", false, true)
    if #found > 0 then
      vim.fn.system(string.format('move "%s" "%s"', found[1], pandoc))
    end
    vim.fn.system('rmdir /s /q "' .. install_dir .. '\\pandoc_tmp" 2>nul')
  else
    -- macOS: unzip
    vim.fn.system(string.format('unzip -jo "%s" "*/bin/pandoc" -d "%s"', archive, install_dir))
  end
  vim.fn.delete(archive)

  if vim.fn.executable(pandoc) == 1 then
    if not is_win then vim.fn.system('chmod +x "' .. pandoc .. '"') end
    return true
  end
  return false
end

--- Typst をインストール
function M._install_typst(install_dir, sysname, machine, is_win, ext)
  -- Typst GitHub Releases の URL
  -- https://github.com/typst/typst/releases/latest/download/
  local fname
  if sysname == "Linux" then
    fname = "typst-x86_64-unknown-linux-musl.tar.xz"
  elseif sysname == "Darwin" then
    fname = (machine == "arm64" or machine == "aarch64")
      and "typst-aarch64-apple-darwin.tar.xz" or "typst-x86_64-apple-darwin.tar.xz"
  else
    fname = "typst-x86_64-pc-windows-msvc.zip"
  end

  local url     = "https://github.com/typst/typst/releases/latest/download/" .. fname
  local archive = install_dir .. "/" .. fname
  local typst   = install_dir .. "/typst" .. ext

  if not M._download(url, archive) then return false end

  -- 解凍
  if fname:match("%.tar%.xz$") then
    vim.fn.system(string.format('tar -xJf "%s" -C "%s" --strip-components=1', archive, install_dir))
  else
    vim.fn.system(string.format('unzip -jo "%s" "*/typst%s" -d "%s"', archive, ext, install_dir))
  end
  vim.fn.delete(archive)

  if vim.fn.executable(typst) == 1 then
    if not is_win then vim.fn.system('chmod +x "' .. typst .. '"') end
    return true
  end
  return false
end

--- cargo を使ってソースからビルドする（リリースバイナリがない場合のフォールバック）
function M._build_from_source()
  if vim.fn.executable("cargo") == 0 then
    vim.notify(
      "[md-preview] ビルド失敗: cargo がインストールされていません。\n"
      .. "Rust をインストールするか、:MdPreviewInstall でリリースバイナリを取得してください。\n"
      .. "Rust インストール: https://rustup.rs",
      vim.log.levels.ERROR
    )
    return false
  end

  local plugin_dir = _plugin_dir
  vim.notify("[md-preview] cargo build --release を実行中...", vim.log.levels.INFO)

  -- download-libs.sh が必要（JS ライブラリを埋め込むため）
  local lib_dir = plugin_dir .. "/src/embedded_lib"
  if vim.fn.isdirectory(lib_dir) == 0 then
    vim.notify("[md-preview] JS ライブラリをダウンロード中...", vim.log.levels.INFO)
    local dl_script = plugin_dir .. "/download-libs.sh"
    if vim.fn.filereadable(dl_script) == 1 then
      vim.fn.system('bash "' .. dl_script .. '"')
      if vim.v.shell_error ~= 0 then
        vim.notify("[md-preview] ライブラリのダウンロードに失敗しました", vim.log.levels.ERROR)
        return false
      end
    end
  end

  local build_result = vim.fn.system(
    string.format('cd "%s" && cargo build --release 2>&1', plugin_dir)
  )
  if vim.v.shell_error ~= 0 then
    vim.notify("[md-preview] ビルド失敗:\n" .. build_result, vim.log.levels.ERROR)
    return false
  end

  local is_win = vim.fn.has("win32") == 1
  local ext = is_win and ".exe" or ""
  local built_binary = plugin_dir .. "/target/release/nvim-md-preview" .. ext
  M.config.binary = built_binary
  vim.notify("[md-preview] ビルド完了: " .. built_binary, vim.log.levels.INFO)
  return true
end

--- セットアップ関数（lazy.nvim などから呼ぶ）
function M.setup(opts)
  M.config = vim.tbl_deep_extend("force", M.config, opts or {})
  M._initialized = true
end

return M
