-- plugin/md_preview.lua
-- Neovim 起動時に自動実行される。コマンドだけを登録する。

-- lazy.nvim 等で setup() を呼ぶ場合はここでは何もしない設計にする
-- ただし、setup() が呼ばれていない場合でもコマンドが使えるようにする

local function ensure_setup()
  local ok, m = pcall(require, "md_preview")
  if not ok then
    vim.notify("[md-preview] モジュールのロードに失敗: " .. m, vim.log.levels.ERROR)
    return nil
  end
  -- setup() がまだ呼ばれていなければデフォルト設定で初期化
  if not m._initialized then
    m.setup({})
  end
  return m
end

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
