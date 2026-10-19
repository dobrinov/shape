// End-to-end test of to-do lists: one from the Blocks palette, ticked with
// a click on its box, an item added with Enter, indented with Tab, and the
// list edited as text.
//   make e2e   (or: cd scripts/e2e && node todo.mjs)
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
const port = 7390 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: Launch\n");
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

  // a list from the palette, named
  await p.locator(".dge-blocks-btn").click(); await p.locator('[data-block="todo"]').click(); await sleep(300);
  await p.keyboard.type("Launch checklist"); await p.keyboard.press("Enter");
  await until(/todo launch-checklist: Launch checklist \{\n  \[ \] First thing\n  \[ \] Second thing\n\}/);

  // a click on the first item's box ticks it
  const first = await p.locator('.dg-node[data-id="launch-checklist.first-thing"]').boundingBox();
  await p.mouse.click(first.x + 6, first.y + first.height / 2); await sleep(400);
  await until(/  \[x\] First thing\n  \[ \] Second thing/);
  assert.match(await p.locator(".dge-canvas").innerText(), /1 of 2 done/);

  // Enter after the second adds an item; Tab makes it a sub-item
  await p.locator('.dg-node[data-id="launch-checklist.second-thing"]').click({ position: { x: 120, y: 10 } }); await sleep(200);
  await p.keyboard.press("Enter"); await sleep(300);
  await p.keyboard.type("On staging"); await p.keyboard.press("Enter"); await sleep(400);
  await until(/  \[ \] Second thing\n  \[ \] On staging\n\}/);
  await p.keyboard.press("Tab"); await sleep(400);
  await until(/  \[ \] Second thing\n    \[ \] On staging\n\}/);

  // edited as text: owner and due date, one more item
  await p.locator('.dg-group[data-id="launch-checklist"]').first().click({ button: "right", position: { x: 30, y: 12 } });
  await p.getByText("Edit as text…").click(); await sleep(200);
  await p.locator(".dge-modal textarea").fill("[x] First thing\n[ ] Second thing [owner: ana, due: 2020-01-01]\n  [ ] On staging\n[ ] Tell support");
  await p.locator(".dge-modal button.primary").click();
  await until(/  \[ \] Second thing \[due: 2020-01-01, owner: ana\]\n    \[ \] On staging\n  \[ \] Tell support\n\}/);
  // the overdue date shows red
  const red = await p.evaluate(() => [...document.querySelectorAll(".dge-canvas text")].some((t) => /1 Jan 2020/.test(t.textContent) && t.getAttribute("font-weight") === "700"));
  assert.ok(red, "an overdue item is marked");
  if (process.env.SHOTS) {
    await p.keyboard.press("Escape"); await p.keyboard.press("f"); await sleep(300);
    await p.screenshot({ path: join(process.env.SHOTS, "todo-light.png") });
    await p.locator('button[title="Light / dark"]').click(); await sleep(300);
    await p.screenshot({ path: join(process.env.SHOTS, "todo-dark.png") });
  }
  assert.deepEqual(errors, []);
  await browser.close();
  console.log("todo: ok");
} catch (e) {
  failed = true;
  console.error(e);
  console.error("\nfile:\n" + saved());
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
