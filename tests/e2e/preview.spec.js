// @ts-check
// プレビューの動作をブラウザで確かめる。各テストは headless の Neovim でプレビューを起動し、
// Neovim を操作しながら画面を確認する
const fs = require("node:fs");
const path = require("node:path");
const { test, expect } = require("@playwright/test");
const { startPreview } = require("./harness");

/** @type {Awaited<ReturnType<typeof startPreview>>} */
let pv;

test.afterEach(async () => {
  await pv?.stop();
  pv = undefined;
});

test("renders Markdown, Mermaid, math, callouts and code", async ({ page }) => {
  pv = await startPreview();
  await page.goto(pv.url);
  const content = page.locator("#content");
  await expect(content.locator("h1")).toHaveText(/Handbook/);
  await expect(content.locator(".mermaid-svg-wrap svg")).toBeVisible();
  await expect(content.locator(".katex").first()).toBeVisible();
  await expect(content.locator("table td").first()).toHaveText("alpha");
  await expect(content.locator("code.hljs.language-rust")).toBeVisible();
  // Tip の callout は GitHub の緑（ライト #1a7f37 / ダーク #3fb950）
  const tip = content.locator(".markdown-alert-tip");
  await expect(tip).toBeVisible();
  const color = await tip.evaluate((el) => getComputedStyle(el).borderLeftColor);
  expect(["rgb(26, 127, 55)", "rgb(63, 185, 80)"]).toContain(color);
});

test("updates while typing in Neovim", async ({ page }) => {
  pv = await startPreview();
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();
  pv.send("3GA Typed from Neovim.<Esc>");
  await expect(page.locator("#content")).toContainText("Typed from Neovim.");
});

test("scrolls with the cursor and jumps to the clicked line", async ({ page }) => {
  pv = await startPreview({ git_diff: false });
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();

  pv.send("G");
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(500);

  // プレビューの段落をクリックすると、Neovim のカーソルがその行へ移る
  pv.send("gg");
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBeLessThan(50);
  const target = page.locator("#content p", { hasText: "Body of section 3 with" });
  await target.scrollIntoViewIfNeeded();
  await target.click();
  const expected = pv.expr('search("Body of section 3 with", "nw")');
  await expect.poll(() => pv.expr('line(".")')).toBe(expected);
});

test("keeps the toolbar visible on long documents", async ({ page }) => {
  pv = await startPreview();
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
  const top = await page.locator("header").evaluate((el) => el.getBoundingClientRect().top);
  expect(top).toBe(0);
});

test("styles content in presentation mode", async ({ page }) => {
  pv = await startPreview();
  await page.goto(pv.url);
  await expect(page.locator("#content .markdown-alert-tip")).toBeVisible();
  await page.locator("#slidesBtn").click();
  const slideTip = page.locator("#slideBody .markdown-alert-tip");
  await expect(slideTip).toBeVisible();
  const inContent = await page.locator("#content .markdown-alert-tip")
    .evaluate((el) => getComputedStyle(el).borderLeftColor);
  // 起動直後の再描画でスライドが作り直されることがあるので、値が出そろうまで待つ
  await expect.poll(() => slideTip.evaluate((el) => getComputedStyle(el).borderLeftColor)).toBe(inContent);
});

test("searches the page", async ({ page }) => {
  pv = await startPreview();
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();
  await expect(page.locator("#searchBar")).toBeHidden();
  await page.locator("#searchBtn").click();
  await page.locator("#searchInput").fill("section 12");
  await expect(page.locator("mark.search-hit").first()).toBeVisible();
  await expect(page.locator("#searchCount")).toHaveText(/^1 \/ \d+$/);
});

test("shows Git changes against HEAD", async ({ page }) => {
  pv = await startPreview();
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();
  pv.send(":3s/the \\*\\*service\\*\\*/the **whole service**/<CR>");
  await expect(page.locator("ins.diff-w")).toContainText("whole");
  await expect(page.locator("#diffSummary")).toHaveText(/1 change vs HEAD/);
  pv.send(":MdPreviewDiff off<CR>");
  await expect(page.locator("#diffNav")).toBeHidden();
  await expect(page.locator("ins.diff-w")).toHaveCount(0);
});

test("limits the Export menu to export.formats", async ({ page }) => {
  pv = await startPreview({ export: { formats: ["pdf", "html"] } });
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();
  await page.locator("#exportBtn").click();
  const visible = page.locator("#exportMenu [data-format]:visible");
  await expect(visible).toHaveCount(2);
  await expect(visible.nth(0)).toHaveAttribute("data-format", "pdf");
  await expect(visible.nth(1)).toHaveAttribute("data-format", "html");
});

test("downloads the Markdown export", async ({ page }) => {
  pv = await startPreview();
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();
  await page.locator("#exportBtn").click();
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.locator('#exportMenu [data-format="markdown"]').click(),
  ]);
  expect(download.suggestedFilename()).toBe("doc.md");
});

test("rejects other sites and paths outside the document folder", async ({ request }) => {
  pv = await startPreview();
  const crossOrigin = await request.get(pv.url + "export?format=markdown", {
    headers: { Origin: "http://evil.example" },
  });
  expect(crossOrigin.status()).toBe(403);
  const absolute = await request.get(pv.url + "assets?p=/etc/hosts");
  expect(absolute.status()).toBe(403);
  const rebinding = await request.get(pv.url, { headers: { Host: "evil.example" } });
  expect(rebinding.status()).toBe(403);
});

test("exports a PDF that looks like the preview", async () => {
  // MDP_TEST_BROWSER で印刷に使うブラウザを指定できる（例: Playwright の chrome-headless-shell）
  const browser = process.env.MDP_TEST_BROWSER;
  pv = await startPreview(browser ? { export: { browser_path: browser } } : {});
  test.skip(pv.expr('luaeval("require(\'md_preview\')._pdf_engine()")') !== "browser",
    "no Chrome, Edge, Brave or Chromium on this machine");

  const out = path.join(pv.dir, "out.pdf");
  pv.send(`:MdPreviewExport pdf ${out}<CR>`);
  await expect.poll(() => (fs.existsSync(out) ? fs.statSync(out).size : 0), { timeout: 90_000 }).toBeGreaterThan(10_000);
  const pdf = fs.readFileSync(out);
  expect(pdf.subarray(0, 5).toString()).toBe("%PDF-");
  // 数式が KaTeX で描画されていれば、KaTeX のフォントが埋め込まれる（Typst の出力では元のテキストのまま）
  expect(pdf.includes("KaTeX")).toBe(true);
});
