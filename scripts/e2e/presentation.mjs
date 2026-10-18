// End-to-end test of presentation mode: slides start from the board's
// sections, their order is drawn on the board, a slide is moved, given notes
// and a transition (all written to the file), then the deck is played.
//   make e2e   (or: cd scripts/e2e && node presentation.mjs)
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
const port = 7190 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "diagram-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: Launch\n\ngroup why: Why {\n  w1: Shoppers retype cards\n}\ngroup how: How {\n  h1: Saved cards\n  h2: Apple Pay\n}\ngroup when: When {\n  t1: Beta in October\n}\nw1 -> h1\nh1 -> t1\n");
const server = spawn(join(root, "target/debug/shape"), [file, "--no-open", "--port", String(port)], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
// saves are debounced: wait until the file matches
const until = async (re) => { for (let i = 0; i < 40 && !re.test(saved()); i++) await sleep(100); assert.match(saved(), re); };
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  const p = await browser.newPage({ viewport: { width: 1400, height: 860 } });
  const errors = [];
  p.on("pageerror", (e) => errors.push(e.message));
  await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
  await p.waitForSelector(".dge-canvas svg"); await sleep(300);

  // presentation mode; start from the board's sections
  await p.locator('[data-act="deck"]').click(); await sleep(300);
  await p.getByRole("button", { name: /Start with the board's sections/ }).click(); await sleep(600);
  await until(/presentation: Launch \{\n  \* \[title: Launch\]\n  why\n  how\n  when\n\}/);
  assert.equal(await p.locator(".dge-slide").count(), 4);
  // the order is drawn on the board: a numbered badge per slide, a path between each
  assert.equal(await p.locator(".dge-deck-num").count(), 4);
  assert.equal(await p.locator(".dge-deck-path").count(), 2); // none to the whole board

  // "when" before "how", notes on "why", "how" zooms in
  await p.locator('.dge-slide[data-slide="3"] button[title="Earlier"]').click(); await sleep(400);
  await until(/  why\n  when\n  how\n/);
  await p.locator('.dge-slide[data-slide="1"] textarea').fill("Start with the pain");
  await p.locator('.dge-slide[data-slide="1"] textarea').blur(); await sleep(400);
  await p.locator('.dge-slide[data-slide="3"] select').selectOption("zoom"); await sleep(400);
  await until(/  why \[notes: Start with the pain\]\n  when\n  how \[transition: zoom\]\n/);
  await p.locator(".dge-deck-settings select").selectOption("fade"); await sleep(400);
  await until(/presentation: Launch \[transition: fade\] \{/);
  if (process.env.SHOTS) {
    await p.screenshot({ path: join(process.env.SHOTS, "deck.png") });
    await p.locator('button[title="Light / dark"]').click(); await sleep(300);
    await p.screenshot({ path: join(process.env.SHOTS, "deck-dark.png") });
    await p.locator('button[title="Light / dark"]').click(); await sleep(200);
  }

  // play it
  await p.locator('[data-act="play"]').click(); await sleep(500);
  assert.match(await p.locator(".dge-present-bar").innerText(), /1 \/ 4 · Launch/);
  await p.keyboard.press("ArrowRight"); await sleep(700);
  assert.match(await p.locator(".dge-present-bar").innerText(), /2 \/ 4 · Why/);
  await p.keyboard.press("n"); await sleep(200);
  assert.equal(await p.locator(".dge-present-notes").innerText(), "Start with the pain");
  if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "deck-play.png") });
  await p.keyboard.press("ArrowRight"); await sleep(700);
  await p.keyboard.press("ArrowRight"); await sleep(1200);
  assert.match(await p.locator(".dge-present-bar").innerText(), /4 \/ 4 · How/);
  await p.keyboard.press("Escape"); await sleep(300);
  assert.equal(await p.locator(".dge-present-bar").count(), 0);
  assert.deepEqual(errors, []);
  await browser.close();
  console.log("presentation: ok");
} catch (e) {
  failed = true;
  console.error(e);
  console.error("\nfile:\n" + saved());
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
