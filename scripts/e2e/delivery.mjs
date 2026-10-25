// End-to-end test of a board without a delivery plan: Delivery says so and
// offers ✦ Plan the delivery, which asks its questions in a form and drafts
// the plan from the answers (a change to keep); Product leaves out what is
// only technical.
//   make e2e   (or: cd scripts/e2e && node delivery.mjs)
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
const port = 7230 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "widget.shape");
writeFileSync(file, `title: Chat widget
perspectives: product

mindmap scope: Chat widget [tags: product] {
  Answers from the help pages
  Hosting [tags: tech]
    Iframe from its own domain
}

adr embed: Embed as an iframe [proposed, tags: tech] {
  context: c
}

adr history: Users can continue past chats [proposed, tags: product] {
  context: c
}
`);
const server = spawn(join(root, "target/debug/shape"), [file, "--no-open", "--port", String(port), "--ai", join(here, "fake-ops.sh")], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
const until = async (re) => { for (let i = 0; i < 40 && !re.test(saved()); i++) await sleep(100); assert.match(saved(), re); };
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  for (const scheme of ["light", "dark"]) {
    const p = await browser.newPage({ viewport: { width: 1300, height: 820 }, colorScheme: scheme });
    const errors = [];
    p.on("pageerror", (e) => errors.push(e.message));
    await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
    await p.waitForSelector(".dge-canvas", { state: "attached" }).catch(async () => { await p.screenshot({ path: "/private/tmp/claude-502/-Users-deyan-dobrinov-dev-personal-diagram/baae2481-7954-4212-9131-821f5926dd42/scratchpad/delivery-fail.png" }); throw new Error("no editor: " + (await p.content()).slice(0, 300)); }); await sleep(800);
    // Product: the scope without what is only technical (a fresh page asks how to view it first)
    if (await p.locator(".dge-persp-card").count()) await p.locator(".dge-persp-card", { hasText: "Product" }).first().click();
    else await p.selectOption(".dge-persp", "product");
    await sleep(600);
    const product = await p.locator(".dge-canvas").textContent();
    assert.match(product, /continue past chats/);
    assert.doesNotMatch(product, /iframe|Hosting/i);
    if (scheme === "light") {
      // every board has Delivery; without a plan it says so, and offers to make one
      await p.selectOption(".dge-persp", "delivery"); await sleep(800);
      assert.match(await p.locator('[data-id="@dash/head/:"]').textContent(), /No delivery plan yet/);
      assert.match(await p.locator(".dge-toast.show").textContent(), /No delivery plan yet/);
      await p.locator(".dge-toast.show .dge-toast-act").click(); await sleep(600);
      // its questions, in a form: answer what you can
      await p.waitForSelector(".dge-plan-form");
      assert.equal(await p.locator(".dge-plan-form textarea").count(), 3);
      assert.match(await p.locator(".dge-plan-form").textContent(), /When does the work start\?/);
      await p.locator(".dge-plan-form textarea").first().fill("2 Nov 2026");
      if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "delivery-questions.png") });
      await p.locator(".dge-dialog button.primary", { hasText: "Draft the plan" }).click();
      // the plan: a change to keep
      await p.waitForSelector(".dge-preview-bar");
      assert.match(await p.locator(".dge-preview-bar").textContent(), /A plan from 2 Nov/);
      await p.locator(".dge-preview-bar button.primary").click(); await sleep(500);
      await until(/group delivery: Delivery plan \[tags: delivery\] \{[\s\S]*gantt plan: Plan \[start: 2026-11-02\]/);
      await p.selectOption(".dge-persp", "delivery"); await sleep(800);
      assert.doesNotMatch(await p.locator('[data-id="@dash/head/:"]').textContent(), /No delivery plan yet/);
      if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "delivery-planned.png") });
    }
    assert.deepEqual(errors, []);
    await p.close();
  }
  await browser.close();
  console.log("delivery e2e: ok");
} catch (e) {
  console.error(e);
  failed = true;
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
