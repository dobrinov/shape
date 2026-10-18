// End-to-end test of the minimap: the board small in the corner, the part
// in view marked; clicking in it moves the view there; M hides it.
//   make e2e   (or: cd scripts/e2e && node minimap.mjs)
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdtempSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const port = 7090 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "diagram-e2e-"));
const file = join(dir, "board.diagram");
writeFileSync(file, "title: Map\n\ngroup a: Left {\n  a1: One\n  a2: Two\n}\ngroup b: Right {\n  b1: Three\n  b2: Four\n}\na1 -> b1\na2 -> b2\nb2 -> far\nfar -> farther\nfarther -> farthest\n");
const server = spawn(join(root, "target/debug/diagram"), [file, "--no-open", "--port", String(port)], { env: { ...process.env, DIAGRAM_WEB_DIR: join(root, "web"), DIAGRAM_NO_UPDATE_CHECK: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  for (const scheme of ["light", "dark"]) {
    const p = await browser.newPage({ viewport: { width: 1300, height: 800 }, colorScheme: scheme });
    const errors = [];
    p.on("pageerror", (e) => errors.push(e.message));
    await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
    await p.waitForSelector(".dge-canvas svg"); await sleep(400);
    assert.ok(await p.locator(".dge-minimap").isVisible());
    assert.equal(await p.locator(".dge-minimap rect.n").count(), 7);
    assert.equal(await p.locator(".dge-minimap rect.g").count(), 2);
    // zoomed in, the view is a small part of the map; clicking moves it
    for (let i = 0; i < 4; i++) await p.keyboard.press("+");
    await sleep(200);
    const before = await p.locator(".dge-minimap-view").boundingBox();
    const map = await p.locator(".dge-minimap").boundingBox();
    assert.ok(before.width < map.width * 0.8, "zoomed in, the view is smaller than the map");
    await p.mouse.click(map.x + map.width - 12, map.y + map.height / 2); await sleep(200);
    const after = await p.locator(".dge-minimap-view").boundingBox();
    assert.ok(after.x > before.x + 20, `the view moved right: ${before.x} → ${after.x}`);
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "minimap-" + scheme + ".png") });
    // M hides it, and that is remembered
    await p.locator(".dge-canvas").click({ position: { x: 700, y: 700 } });
    await p.keyboard.press("m"); await sleep(200);
    assert.ok(!(await p.locator(".dge-minimap").isVisible()));
    await p.reload({ waitUntil: "domcontentloaded" }); await p.waitForSelector(".dge-canvas svg"); await sleep(300);
    assert.ok(!(await p.locator(".dge-minimap").isVisible()));
    await p.keyboard.press("m"); await sleep(200);
    assert.deepEqual(errors, []);
    await p.close();
  }
  await browser.close();
  console.log("minimap: ok");
} catch (e) {
  failed = true;
  console.error(e);
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
