// End-to-end test of images that go to the AI: pasted or picked in Add
// what you know (one ticked to keep on the board) and in Ask; the AI tool
// gets them as files, and the board only keeps the ticked one.
//   make e2e   (or: cd scripts/e2e && node images-ai.mjs)
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
const port = 7590 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: App\n\napi: API\n");
// two small PNGs to give it
const png = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAYAAADED76LAAAAEklEQVR4nGP8z8DwnwEHYMKlAAAzdgL+7x3m1gAAAABJRU5ErkJggg==", "base64");
writeFileSync(join(dir, "slide.png"), png);
writeFileSync(join(dir, "chart.png"), png);
const server = spawn(join(root, "target/debug/shape"), [file, "--no-open", "--port", String(port), "--ai", join(here, "fake-images.sh")], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1", SHAPE_NO_SKILL_REFRESH: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
const until = async (re) => { for (let i = 0; i < 60 && !re.test(saved()); i++) await sleep(100); assert.match(saved(), re); };
let failed = false;
try {
  await sleep(700);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  const p = await browser.newPage({ viewport: { width: 1300, height: 820 } });
  const errors = [];
  p.on("pageerror", (e) => errors.push(e.message));
  await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
  await p.waitForSelector(".dge-canvas svg"); await sleep(400);

  // Add what you know, with two images; one ticked to keep
  await p.locator(".dge-canvas").click({ position: { x: 40, y: 400 } });
  await p.keyboard.press(process.platform === "darwin" ? "Meta+k" : "Control+k"); await sleep(300);
  await p.locator(".dge-modal textarea").fill("The new pricing, from the slides");
  await p.locator(".dge-modal .dge-att-btn input[type=file]").setInputFiles([join(dir, "slide.png"), join(dir, "chart.png")]);
  await sleep(200);
  assert.equal(await p.locator(".dge-modal .dge-att-item").count(), 2);
  if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "images-ai-add.png") });
  // take one off, put it back, keep the first
  await p.locator(".dge-modal .dge-att-x").nth(1).click();
  assert.equal(await p.locator(".dge-modal .dge-att-item").count(), 1);
  await p.locator(".dge-modal .dge-att-btn input[type=file]").setInputFiles([join(dir, "chart.png")]);
  await p.locator(".dge-modal .dge-att-keep input").first().check();
  await p.locator(".dge-modal button.primary").click();
  await p.waitForSelector(".dge-preview-bar"); await sleep(300);
  await p.locator(".dge-preview-bar button", { hasText: "Keep" }).click();
  await until(/seen: 2 images, listed yes/);
  // the ticked one is on the board; the other is not
  assert.equal((saved().match(/^asset /gm) || []).length, 1, "one image kept on the board");

  // Ask, with a pasted screenshot
  await p.locator(".dge-ai-fab").click();
  await p.locator(".dge-ai-menu button", { hasText: "Ask" }).first().click(); await sleep(300);
  await p.evaluate(async (b64) => {
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const dt = new DataTransfer();
    dt.items.add(new File([bytes], "shot.png", { type: "image/png" }));
    const ta = document.querySelector(".dge-ask-panel textarea");
    ta.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }));
  }, png.toString("base64"));
  await sleep(200);
  assert.equal(await p.locator(".dge-ask-panel .dge-att-item").count(), 1);
  if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "images-ai-ask.png") });
  await p.locator(".dge-ask-panel textarea").fill("What is on this?");
  await p.keyboard.press("Enter");
  await p.waitForSelector('.dg-node:has-text("1 images, listed yes")', { timeout: 10000 });
  assert.equal(await p.locator(".dge-ask-panel .dge-att-item").count(), 0, "sent, the thumbnail goes");
  assert.deepEqual(errors, []);
  await browser.close();
  console.log("images-ai: ok");
} catch (e) {
  failed = true;
  console.error(e);
  console.error("\nfile:\n" + saved().slice(0, 600));
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
