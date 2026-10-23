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
const port = 6700 + Math.floor(Math.random() * 90); // (not 6665–6669: Chrome blocks them)
const dir = mkdtempSync(join(tmpdir(), "diagram-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: Launch\ntype: project\n\napi: Orders API [tags: tech, detail: Rust]\nbeta: Open beta [tags: delivery, detail: 28 Oct]\nnote: A loose note\napi -> beta\n\nrisks r: Risks {\n  Fraud [p: 3, i: 4]\n}\n");
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
  assert.deepEqual((await p.locator(".dge-persp-card b").allTextContents()).slice(0, 3), ["Full board", "Delivery", "Product"]);
  // each says what it is for
  assert.match(await p.locator(".dge-persp-card", { hasText: "Delivery" }).innerText(), /Who does what by when/);
  if (process.env.SHOTS) await p.screenshot({ path: process.env.SHOTS + "/picker-roles.png" });
  await p.locator(".dge-persp-card", { hasText: "Full board" }).click(); await sleep(300);

  // a project board's built-in perspectives
  const names = await p.locator(".dge-persp option").allTextContents();
  assert.deepEqual(names.slice(0, 6), ["Full board", "Delivery", "Product", "Technical", "QA", "Legal"]);
  assert.equal(await shown(p), "api,beta,fraud,note");
  assert.ok(!(await p.locator('[data-act="folds"]').isVisible()), "nothing to fold on the full board");
  // Delivery opens on its summary: computed from the board (here: the risk matrix, as it is), read-only; the file does not change
  const before = saved();
  await p.selectOption(".dge-persp", "delivery"); await sleep(400);
  assert.equal(await shown(p), "@dash/head/:,@dash/ri/:fraud");
  assert.match(await p.locator('[data-id="@dash/ri/:fraud"]').textContent(), /Fraud[\s\S]*= 12 \(high\)/);
  assert.ok(!(await p.locator('.dge-tool[data-tool="node"]').isVisible()), "no drawing tools on a summary");
  assert.ok(await p.locator('.dge-tool[data-tool="comment"]').isVisible(), "but comments");
  if (process.env.SHOTS) await p.screenshot({ path: process.env.SHOTS + "/summary.png" });
  // Details: the blocks behind it, as on the board; Summary goes back
  assert.equal(await p.locator('[data-act="folds"]').innerText(), "Details");
  await p.locator('[data-act="folds"]').click(); await sleep(300);
  assert.equal(await shown(p), "beta,fraud");
  assert.equal(await p.locator('[data-act="folds"]').innerText(), "Summary");
  await p.locator('[data-act="folds"]').click(); await sleep(300);
  // a row opens what it stands for on the full board, with a way back
  await p.locator('[data-id="@dash/ri/:fraud"]').click(); await sleep(400);
  assert.equal(await p.locator(".dge-persp").inputValue(), "");
  assert.equal(await p.locator(".dge-back").innerText(), "← Delivery");
  await p.locator(".dge-back").click(); await sleep(400);
  assert.equal(await p.locator(".dge-persp").inputValue(), "delivery");
  assert.ok(!(await p.locator(".dge-back").isVisible()));
  assert.equal(saved(), before);

  // a comment from the summary goes on the risk itself; its pin opens the thread on the board
  await p.locator('.dge-tool[data-tool="comment"]').click();
  await p.locator('[data-id="@dash/ri/:fraud"]').click({ force: true }); await sleep(300);
  await p.locator(".dge-bubble textarea").fill("Who owns this?");
  await p.keyboard.press("Enter"); await sleep(500);
  assert.match(saved(), /comment c1 on fraud by .*: Who owns this\?/);
  assert.equal(await p.locator(".dge-persp").inputValue(), "delivery");
  const pin = p.locator('[data-comment^="@dash/"]').first();
  assert.equal(await pin.count(), 1, "the comment shows on the summary");
  await pin.click({ force: true }); await sleep(600);
  assert.equal(await p.locator(".dge-persp").inputValue(), "");
  assert.match(await p.locator(".dge-bubble").innerText(), /Who owns this\?/);
  await p.keyboard.press("Escape");
  await p.locator(".dge-back").click(); await sleep(400);
  const withComment = saved();

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
  assert.equal(await shown(p), "@dash/head/:,@dash/ri/:fraud");

  // in its details, a box made while looking through it gets its tag, and stays in view
  await p.locator('[data-act="folds"]').click(); await sleep(300);
  assert.equal(await shown(p), "");
  await p.locator('.dge-tool[data-tool="node"]').click();
  await p.mouse.click(600, 400); await sleep(300);
  await p.keyboard.type("Terms"); await p.keyboard.press("Enter"); await sleep(600);
  assert.match(saved(), /: Terms \[tags: support[,\]]/);
  assert.equal((await shown(p)).split(",").length, 1);

  // a new type: warned that the AI will rethink the board
  await p.locator('[data-act="board"]').click();
  await p.locator(".dge-board-menu select").first().selectOption("product"); await sleep(300);
  assert.match(await p.locator(".dge-modal").innerText(), /Use the Product team perspectives\?[\s\S]*Product, Technical, Users[\s\S]*rethink/);
  if (process.env.SHOTS) await p.screenshot({ path: process.env.SHOTS + "/type-change.png" });
  await p.getByRole("button", { name: "Cancel" }).click(); await sleep(200);
  assert.match(saved(), /\ntype: project\n/);
  await p.locator('[data-act="board"]').click();
  await p.locator(".dge-board-menu select").first().selectOption("product"); await sleep(300);
  await p.getByRole("button", { name: "Only change the perspectives" }).click(); await sleep(600);
  assert.match(saved(), /perspectives: product/);
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
