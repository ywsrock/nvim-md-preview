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

test("shows other files as highlighted code with line numbers", async ({ page }) => {
  pv = await startPreview({}, "app.lua");
  await page.goto(pv.url);
  const view = page.locator("#content .code-view");
  await expect(view).toBeVisible();
  await expect(view.locator(".cv-line")).toHaveCount(Number(pv.expr('line("$")')));
  await expect(view.locator(".hljs-keyword").first()).toBeVisible();
  // 複数行にまたがるコメントは、どの行でもコメントの色になる
  await expect(view.locator(".cv-line").nth(3).locator(".hljs-comment")).toHaveText("that spans lines ]]");
  // Markdown 専用の機能は隠れる
  for (const id of ["#tocBtn", "#slidesBtn", "#printBtn", "#exportBtn"]) await expect(page.locator(id)).toBeHidden();
  // クリックした行へ Neovim のカーソルが移る
  await view.locator(".cv-line").nth(6).click();
  await expect.poll(() => pv.expr('line(".")')).toBe("7");
});

test("marks changed lines of code against HEAD", async ({ page }) => {
  pv = await startPreview({}, "app.lua");
  await page.goto(pv.url);
  await expect(page.locator("#content .code-view")).toBeVisible();
  pv.send(':%s/"hello, "/"hi, "/<CR>');
  await expect(page.locator(".cv-line.diff-mod")).toHaveCount(1);
  await expect(page.locator(".diff-ghost summary")).toContainText('return "hello, " .. name');
  await expect(page.locator("#diffSummary")).toHaveText(/1 change vs HEAD/);
});

test("does not follow buffers that cannot be previewed", async ({ page }) => {
  pv = await startPreview({ filetypes: ["markdown"] });
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();
  // filetypes に無い種類のファイルと、ヘルプの画面では切り替わらない
  pv.send(":e app.lua<CR>");
  pv.send(":help<CR>");
  await page.waitForTimeout(800);
  await expect(page.locator("#title")).toHaveText("doc.md");
  await expect(page.locator("#content .code-view")).toHaveCount(0);
});

// 文書の末尾に PlantUML のコードブロックを足す
const addPlantuml = (pv) =>
  pv.send(`:call append(line('$'), ['\`\`\`plantuml', 'Bob -> Alice : hello', '\`\`\`'])<CR>`);

test("renders PlantUML with the local command", async ({ page }) => {
  test.skip(process.platform === "win32", "uses a shell script in place of plantuml");
  // 本物の plantuml の代わりに、受け取ったソースを文字にした SVG を返すスクリプトを使う
  const bin = fs.mkdtempSync(path.join(require("node:os").tmpdir(), "mdp-puml-"));
  const stub = path.join(bin, "plantuml");
  fs.writeFileSync(stub, [
    "#!/bin/sh",
    `printf '<svg xmlns="http://www.w3.org/2000/svg" width="300" height="40"><text x="10" y="25">'`,
    "tr '\\n' ' '",
    "printf '</text></svg>'",
    "",
  ].join("\n"), { mode: 0o755 });
  try {
    pv = await startPreview({ plantuml_path: stub });
    await page.goto(pv.url);
    await expect(page.locator("#content h1")).toBeVisible();
    addPlantuml(pv);
    const img = page.locator("#content .plantuml-block img");
    await expect(img).toBeVisible();
    expect(await img.evaluate((el) => el.naturalWidth)).toBe(300);
    // @startuml / @enduml を補ってから plantuml に渡している
    const src = decodeURIComponent(await img.getAttribute("src") ?? "");
    expect(src).toContain("@startuml Bob -> Alice : hello @enduml");
  } finally {
    fs.rmSync(bin, { recursive: true, force: true });
  }
});

test("uses the PlantUML server only when it is set", async ({ page }) => {
  pv = await startPreview({ plantuml_path: "/nonexistent/plantuml", plantuml_server: "http://127.0.0.1:9/plantuml/" });
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();
  addPlantuml(pv);
  const img = page.locator("#content .plantuml-block img");
  await expect(img).toHaveAttribute("src", /^http:\/\/127\.0\.0\.1:9\/plantuml\/svg\/[0-9A-Za-z_-]+$/);
});

test("shows PlantUML source and a hint without plantuml", async ({ page }) => {
  pv = await startPreview({ plantuml_path: "/nonexistent/plantuml" });
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();
  addPlantuml(pv);
  const block = page.locator("#content .plantuml-block");
  await expect(block.locator(".plantuml-source")).toHaveText(/Bob -> Alice : hello/);
  await expect(block.locator(".plantuml-error")).toContainText("plantuml_server");
  await expect(block.locator("img")).toHaveCount(0);
});

test("renders PlantUML with plantuml from PATH", async ({ page }) => {
  // plantuml -version の終了コードは版によって 0 でないことがあるので、起動できるかだけを見る
  const found = !require("node:child_process").spawnSync("plantuml", ["-version"], { stdio: "ignore" }).error;
  test.skip(!found, "plantuml is not installed");
  pv = await startPreview();
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();
  addPlantuml(pv);
  const img = page.locator("#content .plantuml-block img");
  await expect(img).toBeVisible({ timeout: 30_000 });
  expect(await img.evaluate((el) => el.naturalWidth)).toBeGreaterThan(50);
  expect(decodeURIComponent(await img.getAttribute("src") ?? "")).toContain("Alice");
});

test("opens its own server and refuses a URL meant for another preview", async ({ page }) => {
  pv = await startPreview();
  // ブラウザを開く代わりに URL を記録する
  pv.send(":lua require('md_preview')._open_browser = function(u) vim.g.mdp_url = u end<CR>");
  pv.send(`:lua require('md_preview').config.auto_open = true; require('md_preview')._on_ready(${new URL(pv.url).port})<CR>`);
  await expect.poll(() => pv.expr("get(g:, 'mdp_url', '')")).toMatch(/\/\?instance=[0-9a-f]+$/);
  const own = pv.expr("g:mdp_url");
  await page.goto(own);
  await expect(page.locator("#content h1")).toHaveText(/Handbook/);

  const other = own.replace(/instance=[0-9a-f]+/, "instance=0000");
  const res = await page.goto(other);
  expect(res?.status()).toBe(409);
  await expect(page.locator("h1")).toHaveText("Another preview is using this port");
});

test("opens the browser again when the preview is started again", async ({ page }) => {
  pv = await startPreview();
  await page.goto(pv.url);
  await expect(page.locator("#content h1")).toBeVisible();
  // ブラウザのタブを閉じたあと、サーバーは動いたまま :MdPreview をもう一度実行する
  await page.goto("about:blank");
  pv.send(":lua require('md_preview')._open_browser = function(u) vim.g.mdp_url = u end<CR>");
  pv.send(":lua require('md_preview').config.auto_open = true<CR>");
  pv.send(":MdPreview<CR>");
  await expect.poll(() => pv.expr("get(g:, 'mdp_url', '')")).toMatch(/\/\?instance=[0-9a-f]+$/);
  await page.goto(pv.expr("g:mdp_url"));
  await expect(page.locator("#content h1")).toHaveText(/Handbook/);
});
