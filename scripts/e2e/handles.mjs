// End-to-end test of the box handles and the magnifier (beta feedback): the
// corner grip resizes a box; a click on a side's dot adds a box there; the
// connector's line starts at the dot it was dragged from; Z zooms.
//   make e2e   (or: cd scripts/e2e && node handles.mjs)
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
const port = 7290 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: Handles\n\na: Alpha\nb: Beta\na -> b\n");
const server = spawn(join(root, "target/debug/shape"), [file, "--no-open", "--port", String(port)], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
const until = async (re) => { for (let i = 0; i < 40 && !re.test(saved()); i++) await sleep(100); assert.match(saved(), re); };
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  const p = await browser.newPage({ viewport: { width: 1300, height: 820 } });
  const errors = [];
  p.on("pageerror", (e) => errors.push(e.message));
  await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
  await p.waitForSelector(".dge-canvas svg"); await sleep(300);
  const zoom = async () => parseInt(await p.locator(".dge-zoom-label").innerText(), 10) / 100;

  // select Alpha: its corner grip resizes it
  const a = await p.locator('.dg-node[data-id="a"]').boundingBox();
  await p.mouse.click(a.x + a.width / 2, a.y + a.height / 2); await sleep(200);
  const grip = await p.locator(".dge-grip").boundingBox();
  assert.ok(grip, "a selected box has a corner grip");
  await p.mouse.move(grip.x + grip.width / 2, grip.y + grip.height / 2);
  await p.mouse.down(); await p.mouse.move(grip.x + 80, grip.y + 40, { steps: 6 });
  assert.equal(await p.locator(".dge-size").count(), 1, "the new size is previewed");
  await p.mouse.up();
  await until(/a: Alpha \[[^\]]*w: \d+[^\]]*\]/);
  const a2 = await p.locator('.dg-node[data-id="a"]').boundingBox();
  assert.ok(a2.width > a.width + 40 && a2.height > a.height + 20, `bigger: ${a.width}x${a.height} → ${a2.width}x${a2.height}`);

  // dragging from the right dot: the line starts at that dot, not the centre
  const dots = await p.locator('.dge-handle[data-node="a"]').evaluateAll((cs) => cs.map((c) => c.getBoundingClientRect()).map((r) => ({ x: r.x + r.width / 2, y: r.y + r.height / 2 })));
  const right = dots.reduce((m, d) => (d.x > m.x ? d : m));
  await p.mouse.move(right.x, right.y); await p.mouse.down();
  await p.mouse.move(right.x + 60, right.y + 120, { steps: 5 });
  const start = await p.locator(".dge-rubber").evaluate((el) => { const m = /M([\d.-]+),([\d.-]+)/.exec(el.getAttribute("d")); const pt = el.ownerSVGElement.createSVGPoint(); pt.x = +m[1]; pt.y = +m[2]; const s = pt.matrixTransform(el.getScreenCTM()); return { x: s.x, y: s.y }; });
  assert.ok(Math.abs(start.x - right.x) < 6 && Math.abs(start.y - right.y) < 6, `the line starts at the dot: ${JSON.stringify(start)} vs ${JSON.stringify(right)}`);
  await p.keyboard.press("Escape"); await p.mouse.move(right.x + 400, right.y + 300); await p.mouse.up(); await sleep(300);

  // a click on the right dot adds a box on that side, joined to Alpha, ready to name
  const before = saved();
  await p.mouse.click(a2.x + a2.width / 2, a2.y + a2.height / 2); await sleep(200);
  const dots2 = await p.locator('.dge-handle[data-node="a"]').evaluateAll((cs) => cs.map((c) => c.getBoundingClientRect()).map((r) => ({ x: r.x + r.width / 2, y: r.y + r.height / 2 })));
  const right2 = dots2.reduce((m, d) => (d.x > m.x ? d : m));
  await p.mouse.click(right2.x, right2.y); await sleep(300);
  await p.keyboard.type("Gamma"); await p.keyboard.press("Enter");
  await until(/: Gamma/);
  const id = /\n([\w-]+): Gamma/.exec(saved())[1];
  assert.match(saved(), new RegExp("a -> " + id));
  const g = await p.locator(`.dg-node[data-id="${id}"]`).boundingBox();
  assert.ok(g.x > a2.x + a2.width, "the new box is to the right");
  assert.notEqual(before, saved());

  // dragging a box moves it (smoothly, then it lands)
  const b = await p.locator('.dg-node[data-id="b"]').boundingBox();
  await p.mouse.move(b.x + 20, b.y + b.height / 2); await p.mouse.down();
  await p.mouse.move(b.x + 120, b.y + b.height / 2 + 160, { steps: 12 }); await p.mouse.up();
  await until(/b: Beta \[[^\]]*x: /);

  // Z: the magnifier; a click zooms in, ⌥-click out, Z leaves
  await p.locator(".dge-canvas").click({ position: { x: 1200, y: 760 } });
  await p.keyboard.press("z"); await sleep(100);
  assert.ok(await p.locator(".dge-canvas.zooming").count(), "Z turns the magnifier on");
  const z0 = await zoom();
  await p.mouse.click(650, 400); await sleep(150);
  const z1 = await zoom();
  assert.ok(z1 > z0 * 1.4, `zoomed in: ${z0} → ${z1}`);
  await p.keyboard.down("Alt"); await p.mouse.click(650, 400); await p.keyboard.up("Alt"); await sleep(150);
  assert.ok(Math.abs((await zoom()) - z0) < 0.02, "⌥-click zooms back out");
  // a box drawn with it fills the view
  await p.mouse.move(600, 380); await p.mouse.down(); await p.mouse.move(700, 430, { steps: 4 }); await p.mouse.up(); await sleep(150);
  const z2 = await zoom(); assert.ok(z2 > z0 * 2 || z2 >= 3.9, `a drawn box zooms to it: ${z0} → ${z2}`);
  await p.keyboard.press("z"); await sleep(100);
  assert.equal(await p.locator(".dge-canvas.zooming").count(), 0, "Z again leaves it");

  assert.deepEqual(errors, []);
  await browser.close();
  console.log("handles: ok");
} catch (e) {
  failed = true;
  console.error(e);
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
