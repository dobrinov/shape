// End-to-end test of Board → AI prompts…: the prompts each AI button sends,
// built for the board as it is, to read. Nothing is sent.
//   make e2e   (or: cd scripts/e2e && node prompts.mjs)
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdtempSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const chrome = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const port = 6890 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "diagram-e2e-"));
const file = join(dir, "board.diagram");
writeFileSync(file, "title: Prompts\ntype: product\n\napi: Orders API\nweb: Web app\nweb -> api\ncomment on api: why Rust?\n");
const server = spawn(join(root, "target/debug/diagram"), [file, "--no-open", "--port", String(port)], { env: { ...process.env, DIAGRAM_WEB_DIR: join(root, "web"), DIAGRAM_NO_UPDATE_CHECK: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  for (const scheme of ["light", "dark"]) {
    const p = await browser.newPage({ viewport: { width: 1300, height: 860 }, colorScheme: scheme });
    const errors = []; p.on("pageerror", (e) => errors.push(e.message));
    await p.goto(`http://127.0.0.1:${port}/`); await p.waitForSelector(".dge-canvas svg"); await sleep(400);
    // a product board asks for a perspective first
    await p.locator(".dge-persp-card", { hasText: "Everyone" }).click(); await sleep(200);
    // nothing selected: ⌘⇧C copies the whole board as a prompt to build it
    await p.context().grantPermissions(["clipboard-read", "clipboard-write"], { origin: `http://127.0.0.1:${port}` });
    await p.locator(".dge-canvas").click({ position: { x: 20, y: 300 } });
    await p.keyboard.press(process.platform === "darwin" ? "Meta+Shift+C" : "Control+Shift+C"); await sleep(300);
    const copied = await p.evaluate(() => navigator.clipboard.readText());
    assert.match(copied, /^# Build: Prompts\n[\s\S]*Orders API[\s\S]*why Rust\?/);
    await p.locator('[data-act="board"]').click(); await p.locator('[data-act="prompts"]').click(); await sleep(400);
    const t = await p.locator('textarea[aria-label="Prompt"]').inputValue();
    assert.match(t, /^Today is \d{4}-\d\d-\d\d\./); assert.match(t, /why Rust/);
    for (const [i, re] of [[1, /perspectives/], [2, /Your question/], [3, /What you write/]]) {
      await p.locator(".dge-prompt-tabs button").nth(i).click();
      assert.match(await p.locator('textarea[aria-label="Prompt"]').inputValue(), re);
    }
    await p.locator(".dge-prompt-tabs button").nth(1).click();
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "prompts-" + scheme + ".png") });
    assert.deepEqual(errors, []); await p.close();
  }
  await browser.close(); console.log("prompts ok");
} catch (e) { console.error(e); process.exitCode = 1; } finally { server.kill(); process.exit(); }
