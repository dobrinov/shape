// End-to-end test of the board while the AI works on it: dimmed and
// locked, a card saying what is going on with a timer, Stop waiting
// throws the answer away; when it is done, the changes are shown to keep.
//   make e2e   (or: cd scripts/e2e && node busy.mjs)
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdtempSync, writeFileSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const port = 7690 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: App\n\napi: API\ndb: Database\napi -> db\n");
const server = spawn(join(root, "target/debug/shape"), [file, "--no-open", "--port", String(port), "--ai", join(here, "fake-slow.sh")], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1", SHAPE_NO_SKILL_REFRESH: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
let failed = false;
try {
  await sleep(700);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  for (const scheme of ["light", "dark"]) {
    const p = await browser.newPage({ viewport: { width: 1300, height: 820 }, colorScheme: scheme });
    const errors = [];
    p.on("pageerror", (e) => errors.push(e.message));
    await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
    await p.waitForSelector(".dge-canvas svg"); await sleep(400);
    const before = saved();
    // Rethink: the board waits, locked, and says what is going on
    await p.locator(".dge-ai-fab").click();
    await p.locator(".dge-ai-menu button", { hasText: "Rethink" }).first().click();
    await p.waitForSelector(".dge-busy", { timeout: 3000 });
    assert.match(await p.locator(".dge-busy-card").innerText(), /Rethinking the board[\s\S]*Reading the board/);
    await sleep(1200);
    assert.match(await p.locator(".dge-busy-time").innerText(), /^0:0[1-9]$/);
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "busy-" + scheme + ".png") });
    // nothing on the board can be changed meanwhile
    await p.locator('.dg-node[data-id="api"]').click({ force: true }); await p.keyboard.press("Backspace"); await sleep(200);
    assert.equal(saved(), before);
    if (scheme === "light") {
      // the answer arrives: the board is free again and the changes are shown to keep
      await p.waitForSelector(".dge-busy", { state: "detached", timeout: 8000 });
      await p.waitForSelector(".dge-preview-bar");
      assert.match(await p.locator(".dge-preview-bar").innerText(), /Grouped it/);
      await p.locator(".dge-preview-bar button", { hasText: "Discard" }).click(); await sleep(300);
    } else {
      // Stop waiting: unlocked at once, and the late answer is ignored
      await p.locator(".dge-busy-card button", { hasText: "Stop waiting" }).click(); await sleep(200);
      assert.equal(await p.locator(".dge-busy").count(), 0);
      await sleep(3500);
      assert.equal(await p.locator(".dge-preview-bar").count(), 0, "the late answer is thrown away");
    }
    assert.equal(saved(), before);
    assert.deepEqual(errors, []);
    await p.close();
  }
  await browser.close();
  console.log("busy: ok");
} catch (e) {
  failed = true;
  console.error(e);
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
