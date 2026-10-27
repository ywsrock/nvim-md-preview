-- plugin/md_preview.lua
-- Neovim 起動時に自動実行される。コマンドだけを登録する。

local function ensure_setup()
  local ok, m = pcall(require, "md_preview")
  if not ok then
    vim.notify("[md-preview] モジュールのロードに失敗: " .. m, vim.log.levels.ERROR)
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
end, { desc = "Markdown プレビューを開始" })

vim.api.nvim_create_user_command("MarkdownPreviewStop", function()
  local m = ensure_setup()
  if m then m.stop() end
end, { desc = "Markdown プレビューを停止" })

vim.api.nvim_create_user_command("MarkdownPreviewToggle", function()
  local m = ensure_setup()
  if not m then return end
  if m._is_running() then
    m.stop()
  else
    m.start()
  end
end, { desc = "Markdown プレビューを開始/停止トグル" })

-- 短縮コマンド（後方互換）
vim.api.nvim_create_user_command("MdPreview", function()
  local m = ensure_setup()
  if m then m.start() end
end, { desc = "Markdown プレビューを開始" })

vim.api.nvim_create_user_command("MdPreviewStop", function()
  local m = ensure_setup()
  if m then m.stop() end
end, { desc = "Markdown プレビューを停止" })

vim.api.nvim_create_user_command("MdPreviewInstall", function()
  local ok, m = pcall(require, "md_preview")
  if ok then m.install() end
end, { desc = "nvim-md-preview バイナリをダウンロード" })
