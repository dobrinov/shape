// End-to-end test of revenue forecasts: Blocks → Revenue forecast starts one
// with the board's milestones, Edit as text changes it, and Business leads
// with it (money now, where it gets to).
//   make e2e   (or: cd scripts/e2e && node forecast.mjs)
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
const port = 7490 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: App\ntype: project\n\ngantt road: Roadmap [start: 2026-11-02] {\n  Build [2026-11-02, 3w]\n  Launch [2026-11-23, milestone]\n}\n");
const server = spawn(join(root, "target/debug/shape"), [file, "--no-open", "--port", String(port)], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
const until = async (re) => { for (let i = 0; i < 40 && !re.test(saved()); i++) await sleep(100); assert.match(saved(), re); };
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
    if (await p.locator(".dge-persp-card").count()) { await p.locator(".dge-persp-card", { hasText: "Full board" }).click(); await sleep(200); }
    if (scheme === "light") {
      // Blocks → Revenue forecast: the board's milestone is in it, and its text opens to edit
      await p.locator(".dge-blocks-btn").click(); await p.locator('[data-block="forecast"]').click(); await sleep(400);
      await until(/forecast revenue: Revenue \[unit: £\] \{\n  now: 0\n  Launch \[adds: 2k, ramp: 3\]\n\}/);
      assert.equal(await p.locator(".dge-modal textarea").inputValue(), "now: 0\nLaunch [adds: 2k, ramp: 3]");
      await p.locator(".dge-modal textarea").fill("now: 12k\nLaunch [adds: 6k, ramp: 2]\nPartners [adds: 1.5k, on: 2027-03-01]");
      await p.locator(".dge-modal button.primary").click();
      await until(/  now: 12k\n  Launch \[adds: 6k, ramp: 2\]\n  Partners \[adds: 1.5k, on: 2027-03-01\]\n\}/);
    }
    await p.keyboard.press("f"); await sleep(300);
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "forecast-" + scheme + ".png") });
    // Business leads with it
    await p.locator("select.dge-persp").selectOption("business"); await sleep(600);
    assert.match(await p.locator('[data-id="@dash/head/:"]').textContent(), /Revenue: £12k a month now → £19.5k by /);
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "forecast-business-" + scheme + ".png") });
    assert.deepEqual(errors, []);
    await p.close();
  }
  await browser.close();
  console.log("forecast: ok");
} catch (e) {
  failed = true;
  console.error(e);
  console.error("\nfile:\n" + saved());
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
