-- plugin/md_preview.lua
-- Neovim 起動時に自動実行される。コマンドだけを登録する。

local function ensure_setup()
  local ok, m = pcall(require, "md_preview")
  if not ok then
    vim.notify("[md-preview] Failed to load the module: " .. m, vim.log.levels.ERROR)
    return nil
  end
  if not m._initialized then
    -- g:md_preview_* グローバル変数も読み込む（markdown-preview.nvim 互換）
    m.setup({
      port      = vim.g.md_preview_port or nil,
      auto_open = vim.g.md_preview_auto_open ~= 0,  -- 0 でない限り true
    })
  end
  return m
end

-- MarkdownPreview 系コマンド（markdown-preview.nvim と同じ命名）
vim.api.nvim_create_user_command("MarkdownPreview", function()
  local m = ensure_setup()
  if m then m.start() end
end, { desc = "Start the Markdown preview" })

vim.api.nvim_create_user_command("MarkdownPreviewStop", function()
  local m = ensure_setup()
  if m then m.stop() end
end, { desc = "Stop the Markdown preview" })

vim.api.nvim_create_user_command("MarkdownPreviewToggle", function()
  local m = ensure_setup()
  if not m then return end
  if m._is_running() then
    m.stop()
  else
    m.start()
  end
end, { desc = "Start or stop the Markdown preview" })

-- 短縮コマンド（後方互換）
vim.api.nvim_create_user_command("MdPreview", function()
  local m = ensure_setup()
  if m then m.start() end
end, { desc = "Start the Markdown preview" })

vim.api.nvim_create_user_command("MdPreviewStop", function()
  local m = ensure_setup()
  if m then m.stop() end
end, { desc = "Stop the Markdown preview" })

vim.api.nvim_create_user_command("MdPreviewExport", function(opts)
  local m = ensure_setup()
  if m then m.export(opts.fargs[1], opts.fargs[2]) end
end, {
  nargs = "+",
  desc = "Export the previewed buffer: :MdPreviewExport {pdf|docx|html|epub|markdown} [path]",
  complete = function(arg_lead, cmdline)
    -- 1 つ目の引数は形式、2 つ目はファイルパス
    local argc = #vim.split(vim.trim(cmdline), "%s+") - (cmdline:match("%s$") and 0 or 1)
    if argc >= 2 then return vim.fn.getcompletion(arg_lead, "file") end
    return vim.tbl_filter(function(f) return f:find(arg_lead, 1, true) == 1 end,
      require("md_preview").export_formats)
  end,
})

vim.api.nvim_create_user_command("MdPreviewDiff", function(opts)
  local m = ensure_setup()
  if m then m.toggle_diff(opts.args) end
end, {
  nargs = "?",
  desc = "Show Git changes: :MdPreviewDiff (toggle) / {HEAD|staged|branch} (compare with it) / off",
  complete = function(arg_lead) return require("md_preview")._diff_complete(arg_lead) end,
})

vim.api.nvim_create_user_command("MdPreviewInstall", function()
  local ok, m = pcall(require, "md_preview")
  if ok then m.install() end
end, { desc = "Download the nvim-md-preview binary, pandoc and Typst" })
