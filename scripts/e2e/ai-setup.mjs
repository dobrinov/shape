// End-to-end test of the AI setup help: with no AI connected, ✦ explains
// how to connect one; when the connected one fails (here: not signed in),
// the editor says what went wrong and what to try, and can check it.
//   make e2e   (or: cd scripts/e2e && node ai-setup.mjs)
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdtempSync, writeFileSync, chmodSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const bin = join(root, "target", "debug", "shape");
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const dir = mkdtempSync(join(tmpdir(), "diagram-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: AI\n\napi: Orders API\n");
// a harness that is not signed in
const broken = join(dir, "not-signed-in.sh");
writeFileSync(broken, "#!/bin/sh\necho 'Invalid API key · Please run /login' >&2\nexit 1\n");
chmodSync(broken, 0o755);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const start = (port, ai) => spawn(bin, [file, "--no-open", "--port", String(port)].concat(ai ? ["--ai", ai] : []), {
  env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1", SHAPE_AI: "" },
  stdio: "ignore",
});
const servers = [];
let failed = false;
try {
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  for (const scheme of ["light", "dark"]) {
    // no AI: ✦ → Connect your AI
    const port = 6990 + Math.floor(Math.random() * 100);
    servers.push(start(port, null));
    await sleep(600);
    const p = await browser.newPage({ viewport: { width: 1300, height: 860 }, colorScheme: scheme });
    const errors = [];
    p.on("pageerror", (e) => errors.push(e.message));
    await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
    await p.waitForSelector(".dge-canvas svg"); await sleep(300);
    await p.locator(".dge-ai-fab").click();
    await p.locator(".dge-ai-menu .dge-ai-item", { hasText: "Connect your AI" }).click(); await sleep(500);
    const text = await p.locator(".dge-ai-setup").innerText();
    for (const t of ["Claude Code", "Codex CLI", "Gemini CLI", "codex login", "--ai codex"]) assert.ok(text.includes(t), t);
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "ai-connect-" + scheme + ".png") });
    assert.deepEqual(errors, []);
    await p.close();

    // a harness that fails: what went wrong and what to try
    const port2 = port + 100;
    servers.push(start(port2, broken));
    await sleep(600);
    const q = await browser.newPage({ viewport: { width: 1300, height: 860 }, colorScheme: scheme });
    q.on("pageerror", (e) => errors.push(e.message));
    await q.goto(`http://127.0.0.1:${port2}/`, { waitUntil: "domcontentloaded" });
    await q.waitForSelector(".dge-canvas svg"); await sleep(300);
    await q.locator(".dge-ai-fab").click();
    await q.locator(".dge-ai-menu .dge-ai-item", { hasText: "Rethink" }).click();
    await q.waitForSelector(".dge-ai-error", { timeout: 10000 });
    const why = await q.locator(".dge-ai-setup").innerText();
    assert.match(why, /Invalid API key/);
    assert.match(why, /not signed in/);
    await q.getByRole("button", { name: "Check it works" }).click();
    await q.waitForFunction(() => /did not work/.test(document.querySelector(".dge-ai-setup").innerText), null, { timeout: 10000 });
    if (process.env.SHOTS) await q.screenshot({ path: join(process.env.SHOTS, "ai-failed-" + scheme + ".png") });
    assert.deepEqual(errors, []);
    await q.close();
  }
  await browser.close();
  console.log("ai-setup: ok");
} catch (e) {
  failed = true;
  console.error(e);
} finally {
  servers.forEach((s) => s.kill());
  process.exit(failed ? 1 : 0);
}
