// @ts-check
const { defineConfig } = require("@playwright/test");

module.exports = defineConfig({
  testDir: ".",
  timeout: 60_000,
  // テストごとに Neovim とプレビューのサーバーを起動するので、並列にしない
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: {
    browserName: "chromium",
    viewport: { width: 1100, height: 760 },
    trace: "retain-on-failure",
  },
});
