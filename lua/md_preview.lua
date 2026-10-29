-- lua/md_preview.lua
-- Neovim 側のグルーコード
-- Rust バイナリを起動し、バッファ変更を RPC 経由で送る

local M = {}

-- プラグインの版。Cargo.toml の version と同じにする（CI で照合している）。
-- :MdPreviewInstall はこの版の Release からバイナリを取得する
M.version = "0.8.2"

M._initialized = false

local _plugin_dir = vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":h:h")

-- 設定のデフォルト値
M.config = {
  binary = vim.fn.stdpath("data") .. "/nvim-md-preview/nvim-md-preview"
    .. (vim.fn.has("win32") == 1 and ".exe" or ""),
  port = 9876,
  -- プレビューするファイルの種類（Neovim の filetype）。"*" はすべてのファイル。
  --   Markdown 系は Markdown として描画し、それ以外はハイライト付きのコードとして表示する
  filetypes = { "*" },
  -- これより大きいファイルはプレビューしない（バイト）
  max_file_size = 2 * 1024 * 1024,
  -- 待ち受けアドレス。"0.0.0.0" にすると LAN 内の別端末（タブレットなど）から見られる。
  --   その場合は毎回ランダムなトークンを発行し、トークン付き URL でしか開けない
  host = "127.0.0.1",
  auto_open = true,
  -- アドレスバーやタブの無いアプリ風のウィンドウで開く（Chrome / Edge / Brave / Chromium の --app）。
  --   ブラウザは export.browser_path、無ければ自動で探す。見つからなければ通常どおり開く
  browser_app = false,
  -- カーソル移動に合わせてプレビューをスクロールする
  scroll_sync = true,
  -- 別の Markdown バッファに移動したらプレビュー対象を切り替える
  follow_buffer = true,
  -- 入力が止まってからプレビューを更新するまでの待ち時間（ms）
  debounce_ms = 80,
  -- 初期テーマ: "auto"（OS の設定に従う）| "dark" | "light"
  --   ブラウザのボタンで切り替えた場合はそちらが優先される
  theme = "auto",
  -- 追加の CSS ファイル（組み込みスタイルの後に読み込まれ、上書きできる）
  custom_css = "",
  -- [[page]] / [[page|label]] 形式の Wiki リンクを有効にする
  wiki_links = true,
  -- プレビュー画面のキーボードショートカット（t: 目次 / "/" と Ctrl+F: 検索 / p: プレゼンモード）
  --   Vimium などブラウザの Vim 系拡張とキーがぶつかる場合は false にする（ボタンは使える）
  browser_keys = true,
  -- プレビューのチェックボックス（- [ ] / - [x]）をクリックで切り替え、Neovim のバッファを書き換える。
  --   プレビューが古いときは書き換えない。LAN モード（host が localhost 以外）では使えない
  task_toggle = true,
  -- Neovim のカーソルがあるブロック（段落・見出し・リスト項目・コードの行など）をプレビューで枠で示す
  cursor_highlight = true,
  -- Git の差分をプレビューに表示する（:MdPreviewDiff で切り替え）。Git の管理外のファイルでは何もしない
  git_diff = true,
  -- 比較の基準: "HEAD" | "staged"（ステージ済みの内容）| ブランチ名・タグ・コミット
  git_diff_base = "HEAD",
  -- 差分の色をエディタ（gitsigns / DiffAdd など）に合わせる
  git_diff_colors = true,
  -- PlantUML（```plantuml / ```puml）を描く plantuml コマンドか plantuml.jar。空なら PATH 上の plantuml を使う
  plantuml_path = "",
  -- PlantUML サーバーの URL（例: "https://www.plantuml.com/plantuml"）。設定したときだけ、
  --   図のソースをこのサーバーに送って描く。空ならサーバーは使わない
  plantuml_server = "",
  -- 動作のログを書くファイル（空なら書かない）。起動・停止・ポート・エクスポートの結果・
  --   バイナリのエラーを記録する。文書の内容は書かない
  log_file = "",

  -- エクスポート設定
  export = {
    -- PDF の作り方: "browser"（プレビューを Chrome / Edge / Brave / Chromium で印刷。見た目がプレビューと同じ）
    --   | "typst"。"browser" でブラウザが見つからないときは Typst を使う
    pdf_engine  = "browser",
    -- PDF の印刷に使うブラウザ。空なら Chrome / Edge / Brave / Chromium を探す
    browser_path = "",
    -- DOCX/HTML/EPUB エンジン: "pandoc"。それ以外にすると pandoc をダウンロードしない
    docx_engine = "pandoc",
    -- プレビューの Export メニューと :MdPreviewExport の補完に出す形式（空にすると Export ボタンを隠す）
    formats = { "pdf", "slides", "docx", "html", "epub", "markdown" },
    -- バイナリパス（空の場合は自動ダウンロード先 ~/.local/share/nvim/nvim-md-preview/ を使用）
    pandoc_path = "",
    typst_path  = "",

    -- 出力の体裁（PDF は Typst、DOCX/HTML/EPUB は pandoc に反映）
    paper     = "a4",   -- 用紙サイズ（PDF）: "a4" | "a5" | "us-letter" など Typst の用紙名
    margin    = "2cm",  -- 余白（PDF）
    font      = "",     -- 本文フォント（PDF）。空なら Noto Serif CJK JP などを使う
    font_size = 11,     -- 本文サイズ pt（PDF）
    toc       = false,  -- 目次を入れる
    numbering = false,  -- 見出しに番号を付ける
    -- スライドの PDF（:MdPreviewExport slides）の縦横比: "16:9" | "4:3"
    slide_size = "16:9",
    -- Typst テンプレート（PDF）。`template` 関数を export した .typ ファイル
    --   例: #let template(doc) = { set page(header: [社外秘]); doc }
    template  = "",
  },
}

local state = {
  job_id      = nil,  -- Rust バイナリのジョブID
  chan_id      = nil,  -- RPC チャネルID
  buf_id       = nil,  -- 監視中のバッファ ID
  actual_port  = nil,  -- サーバーが実際に使用しているポート（ポート競合時に変わる）
  update_timer = nil,  -- デバウンス用タイマー
  token        = nil,  -- LAN 公開時のアクセストークン
  git_base     = {},   -- バッファごとの HEAD の内容（false = Git 管理外）
  attached     = {},   -- nvim_buf_attach 済みのバッファ（二重登録防止）
}

local uv = vim.uv or vim.loop

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
-- Markdown として描画する filetype（それ以外はコードとして表示する）
local MARKDOWN_FILETYPES = {
  markdown = true, vimwiki = true, quarto = true, rmd = true, pandoc = true,
  mdx = true, telekasten = true, ["markdown.mdx"] = true, ["markdown.pandoc"] = true,
}

--- バッファの表示の仕方: "markdown" | "code"
function M._view_kind(buf)
  local ft = vim.bo[buf].filetype
  if MARKDOWN_FILETYPES[ft] or MARKDOWN_FILETYPES[ft:match("^[^.]+") or ""] then return "markdown" end
  -- filetype が未検出でも拡張子が Markdown なら Markdown として扱う
  local name = vim.api.nvim_buf_get_name(buf)
  if name:match("%.md$") or name:match("%.markdown$") then return "markdown" end
  return "code"
end

--- バッファをプレビューできるか。できなければ (false, 理由)
--- 通常のファイルのバッファだけを対象にする（ヘルプ・ファイラー・quickfix・ターミナルなどは除く）
function M._previewable(buf)
  if not vim.api.nvim_buf_is_valid(buf) or vim.bo[buf].buftype ~= "" then
    return false, "not a file buffer"
  end
  local name = vim.api.nvim_buf_get_name(buf)
  if name == "" then return false, "the buffer has no file name" end

  local ft = vim.bo[buf].filetype
  local allowed = false
  for _, f in ipairs(M.config.filetypes or {}) do
    if f == "*" or f == ft or f == ft:match("^[^.]+") then allowed = true break end
  end
  if not allowed then
    return false, string.format("filetype %q is not in filetypes", ft)
  end

  local size = vim.api.nvim_buf_get_offset(buf, vim.api.nvim_buf_line_count(buf))
  if size > (M.config.max_file_size or math.huge) then
    return false, string.format("the file is larger than max_file_size (%d bytes)", size)
  end
  -- バイナリのファイル（先頭に NUL を含む）は表示しない
  local f = io.open(name, "rb")
  if f then
    local head = f:read(8192) or ""
    f:close()
    if head:find("\0", 1, true) then return false, "binary file" end
  end
  return true
end

local function send_update(buf)
  if state.chan_id == nil then return end
  local content   = get_buf_content(buf)
  local title     = get_buf_title(buf)
  local base_dir  = get_buf_dir(buf)
  local file_path = vim.api.nvim_buf_get_name(buf)  -- フルパス（エクスポート用）
  local tick      = vim.api.nvim_buf_get_changedtick(buf)
  -- 送った版を覚えておく（チェックボックスの切り替えで、プレビューが最新かを確かめる）
  state.sent = { buf = buf, tick = tick, content = content }
  pcall(vim.rpcnotify, state.chan_id, "md_preview_update", content, title, base_dir, file_path,
    M._view_kind(buf), vim.bo[buf].filetype, buf, tick)
end

--- 最後にブラウザへ送った版の changedtick（テスト用）
function M._sent_tick()
  return state.sent and state.sent.tick or -1
end

--- プレビューのチェックボックスのクリック（バイナリから呼ばれる）
--- line 行目のタスクを checked にする。ブラウザが表示している版（buf, tick）が、
--- 最後に送った版で、かつ今のバッファと同じ内容のときだけ書き換える（古い表示で別の行を書き換えないため）
function M._toggle_task(line, checked, buf, tick)
  if not M.config.task_toggle or not M._is_loopback(M.config.host) then return end
  local function refuse(msg)
    vim.notify("[md-preview] " .. msg, vim.log.levels.WARN)
    M._log("task toggle refused: %s", msg)
  end
  local sent = state.sent
  if buf ~= state.buf_id or not vim.api.nvim_buf_is_valid(buf)
    or not sent or sent.buf ~= buf or sent.tick ~= tick then
    return refuse("The preview is out of date. Try again")
  end
  -- 保存（:w）でも changedtick は増えるので、番号が違っても内容が同じなら最新とみなす
  if vim.api.nvim_buf_get_changedtick(buf) ~= tick and get_buf_content(buf) ~= sent.content then
    return refuse("The preview is out of date. Try again")
  end
  if not vim.bo[buf].modifiable or vim.bo[buf].readonly then
    return refuse("This buffer cannot be modified")
  end
  local text = vim.api.nvim_buf_get_lines(buf, line - 1, line, false)[1] or ""
  -- "- [ ]" / "* [x]" / "1. [ ]"（引用の中の "> - [ ]" も）
  local prefix = text:match("^[%s>]*[-*+]%s+%[") or text:match("^[%s>]*%d+[.)]%s+%[")
  local mark = prefix and text:sub(#prefix + 1, #prefix + 1)
  if not mark or not mark:match("^[ xX]$") or text:sub(#prefix + 2, #prefix + 2) ~= "]" then
    return refuse(string.format("No task on line %d", line))
  end
  if (mark ~= " ") == checked then return end  -- 既にその状態
  vim.api.nvim_buf_set_text(buf, line - 1, #prefix, line - 1, #prefix + 1, { checked and "x" or " " })
  M._log("task toggled: line %d -> %s", line, checked and "done" or "open")
end

--- 比較の基準（git_diff_base）を git show に渡す形にする
--- "HEAD" / ブランチ名 / コミット → "<ref>:./file"、"staged" / "index" → ":./file"（ステージ済みの内容）
local function git_object(base, file)
  if base == "staged" or base == "index" then return ":./" .. file end
  return base .. ":./" .. file
end

--- 比較の基準として使える ref か（staged / index は常に使える）
local function ref_exists(dir, ref)
  if ref == "staged" or ref == "index" then return true end
  vim.fn.system({ "git", "-C", dir, "rev-parse", "--verify", "--quiet", ref .. "^{commit}" })
  return vim.v.shell_error == 0
end

--- 比較の基準となる内容を返す: (内容, 基準, Git のリポジトリ内か)
--- 内容は、基準に無いファイル（新規）なら ""、比較できなければ nil。結果はバッファ・基準ごとにキャッシュする
local function git_base_content(buf)
  local configured = M.config.git_diff_base or "HEAD"
  local key = buf .. "\0" .. configured
  local cached = state.git_base[key]
  if cached ~= nil then return cached.content, cached.base, cached.in_repo end

  local result = { content = nil, base = configured, in_repo = false }
  local name = vim.api.nvim_buf_get_name(buf)
  if name ~= "" and vim.fn.executable("git") == 1 then
    local dir = vim.fn.fnamemodify(name, ":h")
    local file = vim.fn.fnamemodify(name, ":t")
    -- Git の管理外なら何もしない（差分の表示が既定でオンなので、ここでは警告を出さない）
    vim.fn.system({ "git", "-C", dir, "rev-parse", "--is-inside-work-tree" })
    if vim.v.shell_error == 0 then
      result.in_repo = true
      if ref_exists(dir, configured) then
        local out = vim.fn.system({ "git", "-C", dir, "show", git_object(configured, file) })
        -- 基準にファイルが無い（新規ファイル）なら全体を「追加」として扱う
        result.content = vim.v.shell_error == 0 and out or ""
      else
        vim.notify(string.format("[md-preview] Comparison base not found: %s", configured), vim.log.levels.WARN)
      end
    end
  end
  state.git_base[key] = result
  return result.content, result.base, result.in_repo
end

--- ハイライトグループの色（#rrggbb）。specs = { { グループ名, "fg" | "bg" }, ... } の順に探す
local function hl_color(specs)
  for _, spec in ipairs(specs) do
    local ok, hl = pcall(vim.api.nvim_get_hl, 0, { name = spec[1], link = false })
    local c = ok and hl and hl[spec[2]]
    if c then return string.format("#%06x", c) end
  end
  return nil
end

--- エディタの差分の色（gitsigns があればその色）。git_diff_colors = false なら送らない
local function diff_colors()
  if not M.config.git_diff_colors then return {} end
  -- gitsigns と Added などは文字色、DiffAdd などは行の背景色が差分の色
  return {
    add = hl_color({ { "GitSignsAdd", "fg" }, { "Added", "fg" }, { "DiffAdd", "bg" } }),
    mod = hl_color({ { "GitSignsChange", "fg" }, { "Changed", "fg" }, { "DiffChange", "bg" } }),
    del = hl_color({ { "GitSignsDelete", "fg" }, { "Removed", "fg" }, { "DiffDelete", "bg" } }),
  }
end

--- 基準との差分をバイナリに送る
--- hunks の各要素: { s, e, kind, old_md, ctx_s, ctx_e }
---   s..e         変更された行（前後の空行を除く）。kind = "del" のときは削除位置の直前の行（0 = 先頭）
---   old_md       変更前の Markdown。"mod" は前後の段落（空行まで）を含めて切り出す
---   ctx_s..ctx_e old_md に対応する現在の行範囲（単語単位の差分に使う）
local function send_diff(buf)
  if state.chan_id == nil then return end
  local info = { enabled = M.config.git_diff and true or false, base = M.config.git_diff_base or "HEAD" }
  if not M.config.git_diff then
    pcall(vim.rpcnotify, state.chan_id, "md_preview_diff", {}, info)
    return
  end
  local base, used_base, in_repo = git_base_content(buf)
  info.base = used_base
  info.in_repo = in_repo
  info.available = base ~= nil
  info.colors = diff_colors()
  local hunks = {}
  if base then
    local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
    local old_lines = vim.split(base, "\n", { plain = true })
    if old_lines[#old_lines] == "" then table.remove(old_lines) end
    local current = table.concat(lines, "\n") .. "\n"
    local diff = (vim.text and vim.text.diff) or vim.diff
    local raw = diff(base, current, { result_type = "indices" }) or {}
    local function blank(list, n) return (list[n] or ""):match("^%s*$") ~= nil end
    local function slice(list, from, to)
      local out = {}
      for i = math.max(from, 1), math.min(to, #list) do out[#out + 1] = list[i] end
      return table.concat(out, "\n")
    end
    for _, h in ipairs(raw) do
      local start_a, count_a, start_b, count_b = unpack(h)
      -- 前後の空行は除く（空行は直前の段落の範囲に含まれるため、別のブロックに印が付いてしまう）
      local s, e = start_b, start_b + count_b - 1
      while s <= e and blank(lines, s) do s = s + 1 end
      while e >= s and blank(lines, e) do e = e - 1 end
      local a_s, a_e = start_a, start_a + count_a - 1
      while a_s <= a_e and blank(old_lines, a_s) do a_s = a_s + 1 end
      while a_e >= a_s and blank(old_lines, a_e) do a_e = a_e - 1 end

      if count_b == 0 or s > e then
        -- 削除のみ（または空行だけの変更）: 削除された内容を削除位置に表示する
        if a_s <= a_e then
          table.insert(hunks, { start_b, start_b, "del", slice(old_lines, a_s, a_e), start_b, start_b })
        end
      elseif count_a == 0 or a_s > a_e then
        table.insert(hunks, { s, e, "add", "", s, e })
      else
        -- 変更: 前後の段落（空行まで）に広げて切り出す。
        -- 差分の範囲の外側は変更前後で同じ行なので、両方を同じ行数ずつ広げればよい
        local hunk_e = start_b + count_b - 1
        local up, down = 0, 0
        while start_b - up - 1 >= 1 and not blank(lines, start_b - up - 1) do up = up + 1 end
        while hunk_e + down + 1 <= #lines and not blank(lines, hunk_e + down + 1) do down = down + 1 end
        local old_md = slice(old_lines, start_a - up, start_a + count_a - 1 + down)
        table.insert(hunks, { s, e, "mod", vim.trim(old_md), start_b - up, hunk_e + down })
      end
    end
  end
  pcall(vim.rpcnotify, state.chan_id, "md_preview_diff", hunks, info)
end

--- 入力が止まってから send_update する（連続入力で全文送信を繰り返さない）
local function schedule_update(buf)
  if state.update_timer then
    state.update_timer:stop()
  else
    state.update_timer = uv.new_timer()
  end
  state.update_timer:start(M.config.debounce_ms or 0, 0, vim.schedule_wrap(function()
    if state.buf_id == buf and vim.api.nvim_buf_is_valid(buf) then
      send_update(buf)
      send_diff(buf)
    end
  end))
end

--- Rust バイナリにカーソル行を送る（スクロール同期）
local function send_scroll(buf)
  -- カーソル位置はスクロール同期とカーソル行の強調に使う。どちらも無効なら送らない
  if state.chan_id == nil or not (M.config.scroll_sync or M.config.cursor_highlight) then return end
  local line  = vim.api.nvim_win_get_cursor(0)[1]
  local total = vim.api.nvim_buf_line_count(buf)
  pcall(vim.rpcnotify, state.chan_id, "md_preview_scroll",
    math.floor(line), math.floor(total), M.config.scroll_sync and true or false)
end

--- PDF の印刷に使う Chromium 系ブラウザを探す（見つからなければ nil）
function M._find_browser()
  local configured = M.config.export.browser_path
  if configured ~= nil and configured ~= "" then
    local path = vim.fn.expand(configured)
    return vim.fn.executable(path) == 1 and path or nil
  end
  local candidates = {}
  if vim.fn.has("mac") == 1 then
    for _, root in ipairs({ "/Applications", vim.fn.expand("~/Applications") }) do
      for _, app in ipairs({ "Google Chrome", "Microsoft Edge", "Brave Browser", "Chromium" }) do
        table.insert(candidates, string.format("%s/%s.app/Contents/MacOS/%s", root, app, app))
      end
    end
  elseif vim.fn.has("win32") == 1 then
    for _, root in ipairs({ os.getenv("ProgramFiles"), os.getenv("ProgramFiles(x86)"), os.getenv("LOCALAPPDATA") }) do
      if root then
        table.insert(candidates, root .. "\\Google\\Chrome\\Application\\chrome.exe")
        table.insert(candidates, root .. "\\Microsoft\\Edge\\Application\\msedge.exe")
        table.insert(candidates, root .. "\\BraveSoftware\\Brave-Browser\\Application\\brave.exe")
        table.insert(candidates, root .. "\\Chromium\\Application\\chrome.exe")
      end
    end
  end
  for _, name in ipairs({ "google-chrome", "google-chrome-stable", "chromium", "chromium-browser",
    "microsoft-edge", "microsoft-edge-stable", "brave-browser" }) do
    table.insert(candidates, name)
  end
  for _, c in ipairs(candidates) do
    if vim.fn.executable(c) == 1 then return vim.fn.exepath(c) end
  end
  return nil
end

--- PDF を実際にどう作るか: "browser" | "typst"
function M._pdf_engine()
  local engine = M.config.export.pdf_engine
  if engine == "browser" and not M._find_browser() then return "typst" end
  return engine
end

--- エクスポートエンジンのパスを解決してバイナリに送る
--- PlantUML を描くコマンド（plantuml_path、無ければ PATH 上の plantuml）。見つからなければ nil
function M._find_plantuml()
  local configured = M.config.plantuml_path
  if configured ~= "" then
    local path = vim.fn.fnamemodify(vim.fn.expand(configured), ":p")
    if path:match("%.jar$") then
      return vim.fn.filereadable(path) == 1 and path or nil
    end
    return vim.fn.executable(configured) == 1 and vim.fn.exepath(configured) or nil
  end
  if vim.fn.executable("plantuml") == 1 then
    return vim.fn.exepath("plantuml")
  end
  return nil
end

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

  local ex = M.config.export
  local custom_css = ""
  if M.config.custom_css ~= "" then
    local path = vim.fn.expand(M.config.custom_css)
    local ok, lines = pcall(vim.fn.readfile, path)
    if ok then
      custom_css = table.concat(lines, "\n")
    else
      vim.notify(string.format("[md-preview] Cannot read custom_css: %s", path), vim.log.levels.WARN)
    end
  end

  M._log("tools: pandoc=%s typst=%s pdf=%s plantuml=%s", pandoc_path ~= "" and pandoc_path or "-",
    typst_path ~= "" and typst_path or "-", ex.pdf_engine == "browser" and (M._find_browser() or "typst") or ex.pdf_engine,
    M.config.plantuml_server ~= "" and M.config.plantuml_server or (M._find_plantuml() or "-"))
  pcall(vim.rpcnotify, state.chan_id, "md_preview_config", pandoc_path, typst_path, {
    theme      = M.config.theme,
    custom_css = custom_css,
    wiki_links = M.config.wiki_links,
    browser_keys = M.config.browser_keys,
    task_toggle = M.config.task_toggle and M._is_loopback(M.config.host),
    cursor_highlight = M.config.cursor_highlight,
    export_formats = M.config.export.formats,
    plantuml_path = M._find_plantuml() or "",
    plantuml_server = M.config.plantuml_server,
    export = {
      pdf_engine = ex.pdf_engine,
      -- スライドの PDF は pdf_engine に関係なくブラウザで作るので、常に送る
      --   （PDF に使うかどうかはバイナリが pdf_engine を見て決める）
      browser_path = M._find_browser() or "",
      slide_size = ex.slide_size,
      -- :MdPreviewInstall で同梱した Typst のパッケージ（無ければ Typst が初回にネットワークから取得する）
      typst_packages = vim.fn.isdirectory(install_dir .. "/typst-packages") == 1 and (install_dir .. "/typst-packages") or "",
      paper     = ex.paper,
      margin    = ex.margin,
      font      = ex.font,
      font_size = ex.font_size,
      toc       = ex.toc,
      numbering = ex.numbering,
      template  = ex.template ~= "" and vim.fn.fnamemodify(vim.fn.expand(ex.template), ":p") or "",
    },
  })
end

--- バッファの変更イベントを購読する
local function attach_buffer(buf)
  if state.buf_id == buf then return end

  state.buf_id = buf
  M._log("preview: %s", vim.api.nvim_buf_get_name(buf))

  -- テキスト変更イベント（同じバッファに二重登録しない。
  -- 監視対象でなくなったバッファの変更は state.buf_id の比較で無視する）
  if not state.attached[buf] then
    state.attached[buf] = true
    vim.api.nvim_buf_attach(buf, false, {
      on_lines = function(_, changed_buf, _, _, _, _)
        if state.buf_id ~= changed_buf then return end
        schedule_update(changed_buf)
      end,
      on_detach = function(_, detached_buf)
        state.attached[detached_buf] = nil
        if state.buf_id == detached_buf then state.buf_id = nil end
      end,
    })
  end

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
  send_diff(buf)
end

--- Rust バイナリを起動する
--- log_file にログを 1 行追記する（log_file が空なら何もしない）
--- LAN 用のトークンは伏せる（ログをバグ報告に貼っても漏れないように）
function M._log(fmt, ...)
  local file = M.config.log_file
  if file == nil or file == "" then return end
  local msg = select("#", ...) > 0 and string.format(fmt, ...) or fmt
  if state.token then msg = msg:gsub(vim.pesc(state.token), "<token>") end
  local f = io.open(vim.fn.expand(file), "a")
  if not f then return end
  f:write(os.date("%Y-%m-%d %H:%M:%S "), msg, "\n")
  f:close()
end

--- ブラウザで開く URL（サーバーがまだポートを決めていなければ nil）
function M._url()
  if state.job_id == nil or state.actual_port == nil then return nil end
  local url = string.format("http://127.0.0.1:%d/?instance=%s", state.actual_port, state.instance)
  if state.token then url = url .. "&token=" .. state.token end
  return url
end

--- ジョブの出力（on_stdout / on_stderr の data）を行に組み立てる
--- data の最後の要素は改行で終わっていない行の途中で、次の通知の最初の要素に続く。
--- 戻り値: (完成した行の一覧, 行の途中)
function M._join_lines(partial, data)
  local lines = vim.list_slice(data)
  lines[1] = partial .. (lines[1] or "")
  local rest = table.remove(lines)
  return lines, rest
end

local function start_binary()
  if state.job_id ~= nil then
    return true  -- 既に起動済み
  end

  -- stdin/stdout RPC 方式のため --listen は不要
  local cmd = {
    M.config.binary,
    tostring(M.config.port),
    M.config.host,
  }
  state.token = nil
  state.actual_port = nil
  if not M._is_loopback(M.config.host) then
    state.token = M._random_token()
    table.insert(cmd, state.token)
  end

  -- このプレビューの ID。ブラウザで開く URL に付け、別のプレビューのサーバーにつながったら
  --   （WSL と Windows で同じポートを使った場合など）サーバー側で内容を出さないようにする
  state.instance = M._random_token()

  -- 標準エラーの行の途中まで（行は 1 回の通知で全部届くとは限らない）
  local stderr_partial = ""

  -- Rust バイナリを RPC ジョブとして起動
  state.job_id = vim.fn.jobstart(cmd, {
    rpc = true,
    env = { MD_PREVIEW_INSTANCE = state.instance },
    on_exit = function(job_id, exit_code, _)
      -- :MdPreviewStop で止めたジョブや、再起動前の古いジョブの終了は無視する
      --   （ここで状態を消すと、新しく起動したプレビューが動かなくなる）
      if job_id ~= state.job_id then return end
      M._log("the binary exited (exit: %d)", exit_code)
      vim.notify(
        string.format("[md-preview] The binary exited (exit: %d)", exit_code),
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
      local lines
      lines, stderr_partial = M._join_lines(stderr_partial, data)
      for _, line in ipairs(lines) do
        if line ~= "" then
          M._log("binary: %s", line)
          -- READY:<port> でサーバーが使用しているポートを受け取る
          local port_str = line:match("^READY:(%d+)$")
          if port_str then
            state.actual_port = tonumber(port_str)
            vim.schedule(function() M._on_ready(state.actual_port) end)
          end
        end
      end
    end,
  })

  if state.job_id <= 0 then
    vim.notify(string.format("[md-preview] Failed to start the binary: %s", M.config.binary), vim.log.levels.ERROR)
    M._log("failed to start the binary: %s", M.config.binary)
    state.job_id = nil
    return false
  end
  M._log("started: %s (plugin v%s, port %s, host %s)", M.config.binary, M.version, M.config.port, M.config.host)

  state.chan_id = state.job_id
  return true
end

--- プレビューを開始する（:MdPreview コマンドから呼び出す）
function M.start()
  local buf = vim.api.nvim_get_current_buf()
  local ok, reason = M._previewable(buf)
  if not ok then
    vim.notify(string.format("[md-preview] Cannot preview this buffer: %s", reason), vim.log.levels.WARN)
    return
  end

  -- バイナリが存在しなければ自動ダウンロードし、完了後にプレビューを開始する
  if vim.fn.executable(M.config.binary) == 0 then
    vim.notify("[md-preview] Binary not found. Downloading...", vim.log.levels.INFO)
    M.install({
      missing_only = true,
      on_done = function(ok)
        if not ok then
          vim.notify(
            "[md-preview] Automatic download failed. Install it with:\n:MdPreviewInstall",
            vim.log.levels.ERROR
          )
        elseif vim.api.nvim_buf_is_valid(buf) then
          vim.api.nvim_buf_call(buf, M.start)
        end
      end,
    })
    return
  end

  M._check_binary_version()
  -- サーバーが既に動いている（ブラウザのタブを閉じただけ）なら、READY は来ないので後でここから開き直す
  local running = state.job_id ~= nil and state.actual_port ~= nil
  if not start_binary() then return end

  -- 設定（描画オプション・テーマなど）を最初の内容より先に送る。
  -- RPC は順番どおりに届くので、初回描画から設定が反映される
  send_export_config()

  -- バッファを監視開始
  attach_buffer(buf)

  -- 保存後やコミット後は HEAD の内容を取り直す（FocusGained: 外でコミットした場合）
  local follow_group = vim.api.nvim_create_augroup("md_preview_follow", { clear = true })
  vim.api.nvim_create_autocmd({ "BufWritePost", "FocusGained" }, {
    group = follow_group,
    callback = function()
      if not M.config.git_diff or state.buf_id == nil then return end
      state.git_base = {}
      send_diff(state.buf_id)
    end,
  })

  -- 別の Markdown バッファに移動したらプレビュー対象を切り替える
  vim.api.nvim_create_autocmd("BufEnter", {
    group = follow_group,
    callback = function(ev)
      if not M.config.follow_buffer or state.job_id == nil then return end
      if ev.buf == state.buf_id then return end
      -- プレビューできないバッファ（特殊な画面・対象外の種類・巨大なファイル）では切り替えない
      if not M._previewable(ev.buf) then return end
      attach_buffer(ev.buf)
      send_export_config()
      send_scroll(ev.buf)
    end,
  })
  vim.defer_fn(function()
    send_update(buf)
  end, 500)

  -- エンジンパスをバイナリに送る（500ms 後、バイナリ起動完了後）
  vim.defer_fn(function()
    send_export_config()
  end, 600)

  if running then
    M._on_ready(state.actual_port)
  end
end

--- サーバーが待ち受けを始めたら（READY:<port>）ブラウザを開く
--- ポートは WSL で Windows 側の使用中のポートを避けるなどして変わることがあるので、決まってから開く
function M._on_ready(port)
  local url = M._url() or string.format("http://127.0.0.1:%d/?instance=%s", port, state.instance)
  if state.token then
    local lan = M._lan_addresses()
    if #lan > 0 then
      local urls = vim.tbl_map(function(ip)
        return string.format("  http://%s:%d/?token=%s", ip, port, state.token)
      end, lan)
      vim.notify("[md-preview] Open from your LAN:" .. "\n" .. table.concat(urls, "\n"), vim.log.levels.INFO)
    end
  end
  if M.config.auto_open then
    M._open_browser(url)
  end
  vim.notify(string.format("[md-preview] Preview started: %s", url), vim.log.levels.INFO)
end

--- バイナリの版（"0.5.0" など）。--version に対応していない古いバイナリは nil
function M._binary_version(bin)
  local ok, out = pcall(vim.fn.system, { bin or M.config.binary, "--version" })
  if not ok or vim.v.shell_error ~= 0 then return nil end
  return out:match("nvim%-md%-preview (%d+%.%d+%.%d+)")
end

--- バイナリとプラグインの版が違えば :MdPreviewInstall を促す（1 回の起動につき 1 度だけ）
function M._check_binary_version()
  if state.version_checked then return end
  state.version_checked = true
  local v = M._binary_version()
  if v ~= M.version then
    vim.notify(string.format("[md-preview] The binary is %s but the plugin is v%s. Run :MdPreviewInstall to update it.",
      v and ("v" .. v) or "an older version", M.version), vim.log.levels.WARN)
  end
end

function M._is_loopback(host)
  return host == "127.0.0.1" or host == "localhost" or host == "::1"
end

--- 推測できないトークン（32 文字の 16 進数）
function M._random_token()
  local ok, bytes = pcall(uv.random, 16)
  if not ok or type(bytes) ~= "string" then
    math.randomseed(uv.hrtime() % 2147483647)
    bytes = ""
    for _ = 1, 16 do bytes = bytes .. string.char(math.random(0, 255)) end
  end
  return (bytes:gsub(".", function(c) return string.format("%02x", c:byte()) end))
end

--- この端末の LAN の IPv4 アドレス一覧
function M._lan_addresses()
  local list = {}
  for _, addrs in pairs(uv.interface_addresses() or {}) do
    for _, a in ipairs(addrs) do
      if a.family == "inet" and not a.internal then table.insert(list, a.ip) end
    end
  end
  return list
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
  pcall(vim.api.nvim_del_augroup_by_name, "md_preview_follow")
  if state.update_timer then
    state.update_timer:stop()
    state.update_timer:close()
    state.update_timer = nil
  end
  M._log("stopped")
  vim.notify("[md-preview] Preview stopped", vim.log.levels.INFO)
end

--- Git 差分の表示を切り替える（:MdPreviewDiff [base|off]）
--- 引数なし: ON / OFF を切り替える。"off": OFF。それ以外: その基準と比較して ON
function M.toggle_diff(base)
  if base == nil or base == "" then
    M.config.git_diff = not M.config.git_diff
  elseif base == "off" then
    M.config.git_diff = false
  else
    M.config.git_diff = true
    M.config.git_diff_base = base
  end
  state.git_base = {}  -- HEAD などが変わっているかもしれないので取り直す
  if state.buf_id then send_diff(state.buf_id) end
  vim.notify("[md-preview] " .. (M.config.git_diff
    and string.format("Git diff: ON (against %s)", M.config.git_diff_base)
    or "Git diff: OFF"), vim.log.levels.INFO)
end

--- :MdPreviewDiff の補完候補（off / HEAD / staged / ブランチ・タグ）
function M._diff_complete(arg_lead)
  local cands = { "off", "HEAD", "staged" }
  local dir = state.buf_id and vim.fn.fnamemodify(vim.api.nvim_buf_get_name(state.buf_id), ":h") or vim.fn.getcwd()
  local refs = vim.fn.systemlist({ "git", "-C", dir, "for-each-ref", "--format=%(refname:short)", "refs/heads", "refs/tags", "refs/remotes" })
  if vim.v.shell_error == 0 then vim.list_extend(cands, refs) end
  return vim.tbl_filter(function(c) return c:find(arg_lead, 1, true) == 1 end, cands)
end

--- ブラウザでクリックされた行へカーソルを移動する（バイナリから nvim_exec_lua 経由で呼ばれる）
function M._goto_line(line)
  local buf = state.buf_id
  if buf == nil or not vim.api.nvim_buf_is_valid(buf) then return end
  local win = vim.fn.bufwinid(buf)
  if win == -1 then return end
  local total = vim.api.nvim_buf_line_count(buf)
  line = math.max(1, math.min(tonumber(line) or 1, total))
  vim.api.nvim_win_set_cursor(win, { line, 0 })
  -- カーソル行を画面中央に
  vim.api.nvim_win_call(win, function() vim.cmd("normal! zz") end)
end

--- プレビュー内のローカル Markdown リンク / Wiki リンクを開く（バイナリから呼ばれる）
--- path はプレビュー中ファイルからの相対パス
function M._open_link(path)
  local buf = state.buf_id
  if buf == nil or not vim.api.nvim_buf_is_valid(buf) then return end
  local base = vim.fn.fnamemodify(vim.api.nvim_buf_get_name(buf), ":h")
  local target = vim.fn.fnamemodify(base .. "/" .. path, ":p")
  -- Wiki リンク [[Page]] は大文字小文字・空白の違いを許して探す
  if vim.fn.filereadable(target) == 0 then
    local want = vim.fn.fnamemodify(path, ":t"):lower():gsub("[%s_-]", "")
    for _, f in ipairs(vim.fn.globpath(base, "**/*.md", false, true)) do
      if vim.fn.fnamemodify(f, ":t"):lower():gsub("[%s_-]", "") == want then target = f; break end
    end
  end
  if vim.fn.filereadable(target) == 0 then
    vim.notify(string.format("[md-preview] File not found: %s", path), vim.log.levels.WARN)
    return
  end
  local win = vim.fn.bufwinid(buf)
  if win ~= -1 then vim.api.nvim_set_current_win(win) end
  vim.cmd("edit " .. vim.fn.fnameescape(target))
end

--- エクスポート対応形式
M.export_formats = { "pdf", "slides", "docx", "html", "epub", "markdown" }

--- プレビュー中の内容を指定形式で書き出す（:MdPreviewExport コマンドから呼ぶ）
--- path を省略するとプレビュー中ファイルと同じ場所に同じ名前で保存する
function M.export(format, path)
  format = (format or ""):lower()
  if not vim.tbl_contains(M.export_formats, format) then
    vim.notify(string.format("[md-preview] Unsupported format: %s (%s)", format, table.concat(M.export_formats, " / ")), vim.log.levels.ERROR)
    return
  end
  if state.chan_id == nil or state.buf_id == nil then
    vim.notify("[md-preview] The preview is not running. Run :MdPreview first", vim.log.levels.ERROR)
    return
  end
  if M._view_kind(state.buf_id) ~= "markdown" then
    vim.notify("[md-preview] Export is available for Markdown files only", vim.log.levels.ERROR)
    return
  end
  if path == nil or path == "" then
    local name = vim.api.nvim_buf_get_name(state.buf_id)
    if name == "" then
      vim.notify(string.format("[md-preview] Specify where to save: :MdPreviewExport %s <path>", format), vim.log.levels.ERROR)
      return
    end
    -- Markdown は元のファイルと同じ名前にならないよう <name>.export.md にする
    local ext = ({ markdown = "export.md", slides = "slides.pdf" })[format] or format
    path = vim.fn.fnamemodify(name, ":r") .. "." .. ext
  end
  path = vim.fn.fnamemodify(vim.fn.expand(path), ":p")
  -- 編集中のファイルを Neovim の外から上書きしない
  local source = vim.api.nvim_buf_get_name(state.buf_id)
  if source ~= "" and vim.fn.resolve(path) == vim.fn.resolve(vim.fn.fnamemodify(source, ":p")) then
    vim.notify("[md-preview] Export would overwrite the file you are editing. Choose another path, or use :w", vim.log.levels.ERROR)
    return
  end
  -- 最新のバッファ内容を送ってから変換する（未保存の変更も含める）
  send_update(state.buf_id)
  vim.notify(string.format("[md-preview] Converting to %s...", format:upper()), vim.log.levels.INFO)
  M._log("export %s: %s", format, path)
  pcall(vim.rpcnotify, state.chan_id, "md_preview_export", format, path)
end

--- エクスポート完了通知（バイナリから nvim_exec_lua 経由で呼ばれる）
function M._on_export_done(ok, path, err)
  M._log("export %s: %s%s", ok and "done" or "failed", path, ok and "" or ("\n" .. (err or "")))
  if ok then
    vim.notify(string.format("[md-preview] Saved: %s", path), vim.log.levels.INFO)
  else
    vim.notify("[md-preview] Export failed\n" .. (err or ""), vim.log.levels.ERROR)
  end
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

--- cmd.exe に渡す URL の & をエスケープする（?instance=...&token=... がコマンドの区切りにならないように）
function M._cmd_escape(url)
  return (url:gsub("&", "^&"))
end

--- OS・環境に応じてブラウザを開く
--- WSL の場合は Windows 側のブラウザを優先する
function M._open_browser(url)
  vim.fn.jobstart(M._browser_cmd(url), { detach = true })
end

--- アプリ風ウィンドウ（--app）に使う Chromium 系ブラウザ（見つからなければ nil）
--- WSL では Windows 側のブラウザを探す（PDF の印刷用の M._find_browser は Linux 側だけを探す）
function M._find_app_browser()
  local configured = M.config.export.browser_path
  if configured ~= nil and configured ~= "" then return M._find_browser() end
  if detect_wsl() then
    for _, p in ipairs({
      "/mnt/c/Program Files/Google/Chrome/Application/chrome.exe",
      "/mnt/c/Program Files (x86)/Google/Chrome/Application/chrome.exe",
      "/mnt/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
      "/mnt/c/Program Files/Microsoft/Edge/Application/msedge.exe",
      "/mnt/c/Program Files/BraveSoftware/Brave-Browser/Application/brave.exe",
    }) do
      if vim.fn.executable(p) == 1 then return p end
    end
  end
  return M._find_browser()
end

--- ブラウザで url を開くコマンド
function M._browser_cmd(url)
  local cmd

  if M.config.browser_app then
    local browser = M._find_app_browser()
    if browser then return { browser, "--app=" .. url } end
    if not state.app_warned then
      state.app_warned = true
      vim.notify("[md-preview] browser_app: no Chrome, Edge, Brave or Chromium found, so the default browser is used",
        vim.log.levels.WARN)
    end
  end

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
      cmd = { "cmd.exe", "/c", "start", '""', M._cmd_escape(url) }
    else
      cmd = { "xdg-open", url }
    end

  elseif vim.fn.has("unix") == 1 then
    -- 通常の Linux
    cmd = { "xdg-open", url }

  else
    -- Windows (WSL でない)
    cmd = { "cmd.exe", "/c", "start", '""', M._cmd_escape(url) }
  end

  return cmd
end

local uv = vim.uv or vim.loop

local RELEASES = "https://github.com/ywsrock/nvim-md-preview/releases/"

--- ダウンロード元。プラグインと同じ版の Release を優先し、無ければ（未リリースの版など）最新版を使う
local function release_bases()
  return {
    RELEASES .. "download/v" .. M.version .. "/",
    RELEASES .. "latest/download/",
  }
end

--- ダウンロードコマンドを組み立てる（curl → wget → PowerShell の順で使えるものを選ぶ）
local function download_cmd(url, dest)
  if vim.fn.executable("curl") == 1 then
    -- -f: HTTP エラー時は失敗扱い, --progress-bar: stderr に進捗（%）を出す
    return { "curl", "-fL", "--retry", "3", "--progress-bar", "-o", dest, url }
  elseif vim.fn.executable("wget") == 1 then
    return { "wget", "-q", "-O", dest, url }
  elseif vim.fn.has("win32") == 1 then
    -- 進捗表示を切ると Invoke-WebRequest が大幅に速くなる
    return { "powershell", "-NoProfile", "-Command", string.format(
      "$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -Uri '%s' -OutFile '%s' -UseBasicParsing",
      url, dest) }
  end
  return nil
end

--- 非同期ダウンロード。item.pct に進捗を書き込み、完了時に on_done(ok, err) を呼ぶ
local function download_async(item, url, dest, on_done)
  local cmd = download_cmd(url, dest)
  if not cmd then
    return on_done(false, "curl, wget or PowerShell is required")
  end
  item.tmp, item.pct = dest, nil
  local last_err, tail = "", ""
  local job = vim.fn.jobstart(cmd, {
    on_stderr = function(_, data)
      -- stderr は任意の位置で分割されて届くので、末尾をつなげてから最後の "NN.N%" を拾う
      tail = (tail .. table.concat(data, "\n")):sub(-200)
      local pct
      for p in tail:gmatch("(%d+%.?%d*)%%") do pct = tonumber(p) end
      if pct then item.pct = pct end
      for _, line in ipairs(data) do
        if line:match("^curl:") or (line:match("%S") and not line:match("[#%%]")) then
          last_err = vim.trim(line)
        end
      end
    end,
    on_exit = function(_, code)
      item.tmp = nil
      local ok = code == 0 and vim.fn.getfsize(dest) > 0
      if not ok then os.remove(dest) end
      on_done(ok, not ok and (last_err ~= "" and last_err or ("exit " .. code)) or nil)
    end,
  })
  if job <= 0 then
    on_done(false, string.format("Cannot start the download process: %s", cmd[1]))
  end
end

--- tar.gz を dir に展開する（Windows 10 以降は tar.exe が標準搭載）
local function extract_async(archive, dir, on_done)
  -- cwd を展開先にして相対パスで渡す（GNU tar が "C:\..." をリモートホストと解釈するのを避ける）
  local job = vim.fn.jobstart({ "tar", "-xzf", vim.fn.fnamemodify(archive, ":t") }, {
    cwd = dir,
    on_exit = function(_, code) on_done(code == 0) end,
  })
  if job <= 0 then on_done(false) end
end

--- 実行権限を付けて実行可能か確認する
local function finalize(path, is_win)
  if not is_win then uv.fs_chmod(path, 493) end  -- 0755
  return vim.fn.executable(path) == 1
end

--- 1 項目をインストールする
--- item.asset の .tar.gz を優先し、無ければ（または tar が無ければ）非圧縮版を取得する
local function install_item(item, install_dir, is_win, on_done)
  local function fail(err)
    item.status, item.err = "fail", err
    on_done()
  end
  local function succeed()
    -- ディレクトリとして展開するもの（Typst のパッケージ）は、できていれば成功
    if item.dir then
      if vim.fn.isdirectory(item.dest) == 1 then
        item.status = "done"
        return on_done()
      end
      return fail(string.format("Not found after extracting: %s", item.dest))
    end
    if finalize(item.dest, is_win) then
      item.status = "done"
      on_done()
    else
      fail(string.format("Not executable: %s", item.dest))
    end
  end

  local bases = release_bases()

  -- bases[i] から取得する。失敗したら次のダウンロード元へ
  local function try_base(i, last_err)
    local base = bases[i]
    if not base then return fail(last_err or "download failed") end
    item.url = base .. item.asset

    local function fetch_raw()
      local tmp = item.dest .. ".download"
      download_async(item, base .. item.asset, tmp, function(ok, err)
        if not ok then return try_base(i + 1, err) end
        local renamed, rename_err = uv.fs_rename(tmp, item.dest)
        if not renamed then
          os.remove(tmp)
          return fail(rename_err)
        end
        succeed()
      end)
    end

    if not item.archive or vim.fn.executable("tar") == 0 then
      return fetch_raw()
    end

    local archive = install_dir .. "/" .. item.asset .. ".tar.gz"
    download_async(item, base .. item.asset .. ".tar.gz", archive, function(ok)
      -- 古いリリースには .tar.gz が無いので非圧縮版にフォールバック
      if not ok then return fetch_raw() end
      item.status = "extract"
      extract_async(archive, install_dir, function(extracted)
        os.remove(archive)
        if extracted then succeed() else fetch_raw() end
      end)
    end)
  end

  try_base(1)
end

--- 進捗をコマンドラインに 1 行で表示する（ASCII のみ：表示幅を超えると Hit-Enter が出るため）
local function render_progress(items)
  local parts = {}
  for _, it in ipairs(items) do
    local s
    if it.status == "done" then
      s = "done"
    elseif it.status == "fail" then
      s = "FAILED"
    elseif it.status == "extract" then
      s = "extracting"
    elseif it.pct then
      s = string.format("%d%%", it.pct)
    else
      local size = it.tmp and vim.fn.getfsize(it.tmp) or -1
      s = size > 0 and string.format("%.1fMB", size / 1048576) or "..."
    end
    parts[#parts + 1] = it.label .. " " .. s
  end
  local msg = "[md-preview] installing: " .. table.concat(parts, " | ")
  vim.api.nvim_echo({ { msg:sub(1, math.max(vim.v.echospace - 1, 1)) } }, false, {})
end

--- GitHub Releases から OS に合わせたバイナリを並列ダウンロードする
--- lazy.nvim の build オプションや :MdPreviewInstall コマンドから呼ぶ
--- opts.missing_only: 既にあるものはスキップ / opts.on_done(ok): 完了時コールバック
--- UI が無い（headless）場合は完了まで待って ok を返す
function M.install(opts)
  opts = opts or {}
  if M._installing then
    vim.notify("[md-preview] Installation already in progress", vim.log.levels.WARN)
    return false
  end

  local uname   = uv.os_uname()
  local sysname = uname.sysname  -- "Linux" / "Darwin" / "Windows_NT"
  local machine = uname.machine  -- "x86_64" / "arm64" / "aarch64"
  local is_arm  = machine == "arm64" or machine == "aarch64"

  local is_win = sysname:match("Windows") ~= nil or vim.fn.has("win32") == 1
  local ext    = is_win and ".exe" or ""
  local install_dir = vim.fn.stdpath("data") .. "/nvim-md-preview"
  vim.fn.mkdir(install_dir, "p")

  local target, plat
  if sysname == "Linux" then
    target = is_arm and "aarch64-unknown-linux-gnu" or "x86_64-unknown-linux-gnu"
    plat   = is_arm and "linux-arm64" or "linux"
  elseif sysname == "Darwin" then
    target = is_arm and "aarch64-apple-darwin" or "x86_64-apple-darwin"
    plat   = is_arm and "macos-arm64" or "macos-x86_64"
  else
    target, plat = "x86_64-pc-windows-msvc", "windows"
  end

  local cfg = M.config.export
  local items = {
    { label = "core", name = "nvim-md-preview", asset = "nvim-md-preview-" .. target .. ext,
      dest = install_dir .. "/nvim-md-preview" .. ext },
  }
  if cfg.docx_engine == "pandoc" then
    items[#items + 1] = { label = "pandoc", name = "pandoc", asset = "pandoc-" .. plat .. ext,
      dest = install_dir .. "/pandoc" .. ext, archive = true }
  end
  -- Typst は PDF を Typst で作るとき（"browser" でブラウザが無いときも）だけ入れる。
  -- パッケージ（cmarker）も一緒に入れて、PDF を作るときにネットワークへ取りに行かないようにする
  if cfg.pdf_engine == "typst" or (cfg.pdf_engine == "browser" and not M._find_browser()) then
    items[#items + 1] = { label = "typst", name = "Typst", asset = "typst-" .. plat .. ext,
      dest = install_dir .. "/typst" .. ext, archive = true }
    items[#items + 1] = { label = "pkgs", name = "Typst packages", asset = "typst-packages",
      dest = install_dir .. "/typst-packages", archive = true, dir = true }
  end
  for _, it in ipairs(items) do
    -- .tar.gz のアセット名には .exe を付けない（pandoc-windows.tar.gz）
    if it.archive then it.asset = it.asset:gsub("%.exe$", "") end
    local present = it.dir and vim.fn.isdirectory(it.dest) == 1 or vim.fn.executable(it.dest) == 1
    if opts.missing_only and present then it.status = "done" end
  end

  M._installing = true
  local timer = uv.new_timer()
  timer:start(0, 200, vim.schedule_wrap(function() render_progress(items) end))

  local finished, result = false, false
  local function on_all_done()
    timer:stop()
    timer:close()
    M._installing = false

    local core = items[1]
    local lines, level = {}, vim.log.levels.INFO
    for _, it in ipairs(items) do
      if it.status == "done" then
        lines[#lines + 1] = "OK      " .. it.name .. ": " .. it.dest
      else
        lines[#lines + 1] = "FAILED  " .. it.name .. ": " .. (it.err or "unknown error")
          .. "\n   URL: " .. (it.url or (release_bases()[1] .. it.asset))
        level = vim.log.levels.WARN
      end
    end
    result = core.status == "done"
    if result then
      M.config.binary = core.dest
    else
      level = vim.log.levels.ERROR
    end
    vim.api.nvim_echo({ { "" } }, false, {})
    vim.notify("[md-preview] " .. (level == vim.log.levels.INFO and "Installation complete"
      or "Installation result")
      .. "\n" .. table.concat(lines, "\n"), level)
    finished = true
    if opts.on_done then opts.on_done(result) end
  end

  local pending = 0
  for _, it in ipairs(items) do
    if it.status ~= "done" then pending = pending + 1 end
  end
  if pending == 0 then
    on_all_done()
  else
    for _, it in ipairs(items) do
      if it.status ~= "done" then
        install_item(it, install_dir, is_win, function()
          pending = pending - 1
          if pending == 0 then on_all_done() end
        end)
      end
    end
  end

  if #vim.api.nvim_list_uis() == 0 then
    vim.wait(30 * 60 * 1000, function() return finished end, 200)
    return result
  end
  return true
end

--- プレビューの状態（:MdPreviewInfo で表示する内容）
function M.info()
  local buf = state.buf_id
  local file = (buf and vim.api.nvim_buf_is_valid(buf)) and vim.api.nvim_buf_get_name(buf) or nil
  local plantuml
  if M.config.plantuml_server ~= "" then
    plantuml = "server " .. M.config.plantuml_server
  else
    plantuml = M._find_plantuml() or "not found"
  end
  return {
    running = state.job_id ~= nil,
    url = M._url(),
    port = state.actual_port,
    instance = state.instance,
    file = file ~= "" and file or nil,
    kind = buf and vim.api.nvim_buf_is_valid(buf) and M._view_kind(buf) or nil,
    git_diff = M.config.git_diff and M.config.git_diff_base or false,
    pdf_engine = M._pdf_engine(),
    plantuml = plantuml,
    log_file = M.config.log_file ~= "" and vim.fn.expand(M.config.log_file) or nil,
    version = M.version,
  }
end

--- :MdPreviewInfo [copy] — 状態を表示する。copy なら URL をクリップボードにコピーする
function M.show_info(arg)
  local i = M.info()
  if arg == "copy" then
    if not i.url then
      vim.notify("[md-preview] The preview is not running", vim.log.levels.WARN)
      return
    end
    vim.fn.setreg("+", i.url)
    vim.fn.setreg('"', i.url)
    vim.notify("[md-preview] Copied: " .. i.url, vim.log.levels.INFO)
    return
  end
  local rows = {
    { "Status", i.running and "running" or "stopped" },
    { "URL", i.url or "-" },
    { "File", i.file and (i.file .. " (" .. (i.kind or "?") .. ")") or "-" },
    { "Git diff", i.git_diff and ("vs " .. i.git_diff) or "off" },
    { "PDF", i.pdf_engine },
    { "PlantUML", i.plantuml },
    { "Log file", i.log_file or "off" },
    { "Version", "v" .. i.version },
  }
  local lines = { "[md-preview]" }
  for _, r in ipairs(rows) do table.insert(lines, string.format("  %-9s %s", r[1], r[2])) end
  vim.notify(table.concat(lines, "\n"), vim.log.levels.INFO)
end

--- セットアップ関数（lazy.nvim などから呼ぶ）
function M.setup(opts)
  M.config = vim.tbl_deep_extend("force", M.config, opts or {})
  M._initialized = true
end

return M
