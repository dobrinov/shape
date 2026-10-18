// End-to-end test of perspectives in a real browser: a project board's
// built-in ones, what one shows, a new one written to the board, the pick
// kept on reload, and new elements tagged so they stay in view.
//   make e2e   (or: cd scripts/e2e && node perspectives.mjs)
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
const port = 6590 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "diagram-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: Launch\ntype: project\n\napi: Orders API [tags: tech, detail: Rust]\nbeta: Open beta [tags: exec, detail: 28 Oct]\nnote: A loose note\napi -> beta\n\nrisks r: Risks {\n  Fraud [p: 3, i: 4]\n}\n");
const server = spawn(bin, [file, "--no-open", "--port", String(port)], {
  env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1" },
  stdio: "ignore",
});
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
const shown = (p) => p.evaluate(() => [...document.querySelectorAll(".dge-canvas .dg-node")].map((n) => n.getAttribute("data-id")).sort().join(","));
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  const p = await browser.newPage({ viewport: { width: 1300, height: 800 } });
  const errors = [];
  p.on("pageerror", (e) => errors.push(e.message));
  await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
  await p.waitForSelector(".dge-canvas svg"); await sleep(300);

  // opening asks whose eyes to look through
  await p.waitForSelector(".dge-persp-card");
  assert.deepEqual((await p.locator(".dge-persp-card b").allTextContents()).slice(0, 3), ["Everyone", "Senior management", "Team"]);
  await p.locator(".dge-persp-card", { hasText: "Everyone" }).click(); await sleep(300);

  // a project board's built-in perspectives
  const names = await p.locator(".dge-persp option").allTextContents();
  assert.deepEqual(names.slice(0, 5), ["Everyone", "Senior management", "Team", "Product", "Technical"]);
  assert.equal(await shown(p), "api,beta,fraud,note");
  // Senior management: what is tagged for it and the risks, titles only; the file does not change
  const before = saved();
  await p.selectOption(".dge-persp", "exec"); await sleep(400);
  assert.equal(await shown(p), "beta,fraud");
  assert.equal(await p.locator('.dg-node[data-id="beta"]').textContent(), "Open beta");
  assert.equal(saved(), before);

  // a new perspective is written to the board and picked; kept on reload
  await p.selectOption(".dge-persp", "__new");
  await p.locator(".dge-persp-form input").first().fill("Support");
  await p.locator(".dge-modal button.primary").click(); await sleep(600);
  assert.match(saved(), /\nperspective support: Support \[tags: support\]\n/);
  assert.equal(await p.locator(".dge-persp").inputValue(), "support");
  await p.reload(); await p.waitForSelector(".dge-canvas svg"); await sleep(500);
  // the one picked last time is marked
  await p.waitForSelector(".dge-persp-card.on");
  assert.match(await p.locator(".dge-persp-card.on").innerText(), /Support[\s\S]*last time/);
  await p.locator(".dge-persp-card.on").click(); await sleep(300);
  assert.equal(await p.locator(".dge-persp").inputValue(), "support");
  assert.equal(await shown(p), "");

  // a box made while looking through it gets its tag, and stays in view
  await p.locator('.dge-tool[data-tool="node"]').click();
  await p.mouse.click(600, 400); await sleep(300);
  await p.keyboard.type("Terms"); await p.keyboard.press("Enter"); await sleep(600);
  assert.match(saved(), /: Terms \[tags: support[,\]]/);
  assert.equal((await shown(p)).split(",").length, 1);

  // a new type: warned that the AI will rethink the board
  await p.locator('[data-act="board"]').click();
  await p.locator(".dge-board-menu select").first().selectOption("product"); await sleep(300);
  assert.match(await p.locator(".dge-modal").innerText(), /Make this a Product board\?[\s\S]*Product, Technical, User[\s\S]*rethink/);
  if (process.env.SHOTS) await p.screenshot({ path: process.env.SHOTS + "/type-change.png" });
  await p.getByRole("button", { name: "Cancel" }).click(); await sleep(200);
  assert.match(saved(), /type: project/);
  await p.locator('[data-act="board"]').click();
  await p.locator(".dge-board-menu select").first().selectOption("product"); await sleep(300);
  await p.getByRole("button", { name: "Only change the type" }).click(); await sleep(600);
  assert.match(saved(), /type: product/);
  assert.deepEqual(errors, []);
  await browser.close();
  console.log("ok — perspectives e2e passed");
} catch (e) {
  failed = true;
  console.error(e.message || e);
  console.error("\nfile:\n" + saved());
} finally {
  server.kill();
}
process.exit(failed ? 1 : 0);
