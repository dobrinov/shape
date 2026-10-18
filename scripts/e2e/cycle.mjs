// End-to-end test of cycles in a real browser: one added from the Blocks
// palette, edited as text, a step added after another.
//   make e2e   (or: cd scripts/e2e && node cycle.mjs)
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdtempSync, writeFileSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const bin = join(root, "target", "debug", "shape");
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const port = 6690 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "diagram-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: Loops\n");
const server = spawn(bin, [file, "--no-open", "--port", String(port)], {
  env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1" },
  stdio: "ignore",
});
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  const p = await browser.newPage({ viewport: { width: 1300, height: 800 } });
  const errors = [];
  p.on("pageerror", (e) => errors.push(e.message));
  await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
  await p.waitForSelector(".dge-canvas svg"); await sleep(300);

  await p.locator(".dge-blocks-btn").click(); await p.locator('[data-block="cycle"]').click(); await sleep(300);
  await p.keyboard.type("Build loop"); await p.keyboard.press("Enter"); await sleep(600);
  assert.match(saved(), /cycle build-loop: Build loop \{\n  Plan\n  Do\n  Check\n  Act\n\}/);

  // edit as text: reorder, rename, add
  const g = await p.locator('.dg-group[data-id="build-loop"]').first().boundingBox();
  await p.mouse.click(g.x + 20, g.y + 8, { button: "right" });
  await p.locator(".dge-menu button", { hasText: "Edit as text…" }).click();
  await p.locator(".dge-modal textarea").fill("Plan\nBuild\nMeasure\nLearn\nShare");
  await p.locator(".dge-modal button.primary").click(); await sleep(600);
  assert.match(saved(), /cycle build-loop: Build loop \{\n  Plan\n  Build\n  Measure\n  Learn\n  Share\n\}/);

  // a step after another
  await p.locator('.dg-node[data-id="build-loop.plan"]').click({ button: "right" });
  await p.locator(".dge-menu button", { hasText: "Add step after…" }).click(); await sleep(200);
  await p.keyboard.type("Prioritise"); await p.keyboard.press("Enter"); await sleep(600);
  assert.match(saved(), /  Plan\n  Prioritise\n  Build\n/);
  assert.equal(await p.locator('.dg-group[data-id="build-loop"] ~ * .dg-node, .dg-node[data-id^="build-loop."]').count() >= 6, true);
  assert.deepEqual(errors, []);
  await p.screenshot({ path: join(dir, "cycle.png") });
  await browser.close();
  console.log("ok — cycle e2e passed (screenshot: " + join(dir, "cycle.png") + ")");
} catch (e) {
  failed = true;
  console.error(e.message || e);
  console.error("\nfile:\n" + saved());
} finally {
  server.kill();
}
process.exit(failed ? 1 : 0);
