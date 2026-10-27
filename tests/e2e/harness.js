// テスト用に headless の Neovim でプレビューを起動し、外から操作するための道具
const { spawn, execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const ROOT = path.resolve(__dirname, "../..");
const BINARY = process.env.MD_PREVIEW_BINARY
  || path.join(ROOT, "target", "release", process.platform === "win32" ? "nvim-md-preview.exe" : "nvim-md-preview");

// 見本の文書。HEAD にコミットした版から、テストの中で編集して差分を作る
const DOC = `# Handbook

Intro paragraph about the **service**.

> [!TIP]
> Keep retries idempotent.

## Diagram

\`\`\`mermaid
flowchart LR
  A --> B
\`\`\`

## Math

Inline $e^{i\\pi} + 1 = 0$ and a table:

| Name | Value |
|---|---:|
| alpha | 1 |

## Code

\`\`\`rust
fn main() {}
\`\`\`

${Array.from({ length: 40 }, (_, i) => `## Section ${i + 1}\n\nBody of section ${i + 1} with some words.\n`).join("\n")}
## Last

The final paragraph.
`;

let nextPort = 9950;

/** 一時ディレクトリに Git リポジトリと文書を作り、Neovim でプレビューを始める */
async function startPreview(options = {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "mdp-e2e-"));
  const git = (...args) => execFileSync("git", args, { cwd: dir, stdio: "ignore" });
  git("init", "-q", "-b", "main");
  fs.writeFileSync(path.join(dir, "doc.md"), DOC);
  git("add", "doc.md");
  git("-c", "user.name=test", "-c", "user.email=test@example.com", "commit", "-qm", "init");

  const port = nextPort++;
  const socket = process.platform === "win32"
    ? `\\\\.\\pipe\\mdp-e2e-${process.pid}-${port}`
    : path.join(dir, "nvim.sock");
  const setup = {
    binary: BINARY,
    port,
    auto_open: false,
    debounce_ms: 20,
    ...options,
  };
  const lua = `
    local m = require("md_preview")
    m.setup(vim.json.decode(${JSON.stringify(JSON.stringify(setup))}))
    vim.cmd("edit " .. vim.fn.fnameescape(${JSON.stringify(path.join(dir, "doc.md"))}))
    vim.bo.filetype = "markdown"
    m.start()
  `;
  const luaFile = path.join(dir, "start.lua");
  fs.writeFileSync(luaFile, lua);

  const nvim = spawn("nvim", [
    "--headless", "--listen", socket, "-u", "NONE",
    "--cmd", `set rtp^=${ROOT} noswapfile`,
    "-c", "runtime plugin/md_preview.lua",
    "-c", `luafile ${luaFile}`,
  ], { stdio: "ignore", env: { ...process.env, XDG_DATA_HOME: path.join(dir, "data") } });

  const url = `http://127.0.0.1:${port}/`;
  await waitFor(async () => (await fetch(url)).ok, 15_000);

  const remote = (...args) => execFileSync("nvim", ["--server", socket, ...args], { encoding: "utf8" });
  return {
    dir,
    url,
    /** Neovim にキーを送る（:Command<CR> も可） */
    send: (keys) => remote("--remote-send", keys),
    /** Neovim の式を評価した結果 */
    expr: (e) => remote("--remote-expr", e).trim(),
    async stop() {
      try { remote("--remote-send", "<Esc>:qa!<CR>"); } catch {}
      await new Promise((resolve) => { nvim.once("exit", resolve); setTimeout(resolve, 3000); });
      nvim.kill();
      fs.rmSync(dir, { recursive: true, force: true });
    },
  };
}

async function waitFor(check, timeout) {
  const end = Date.now() + timeout;
  for (;;) {
    try { if (await check()) return; } catch {}
    if (Date.now() > end) throw new Error("timed out waiting for the preview server");
    await new Promise((r) => setTimeout(r, 200));
  }
}

module.exports = { startPreview };
