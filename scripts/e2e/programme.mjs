// End-to-end test of a board of boards: right-click → Bring in a board… makes
// other .shape files canvases of this one, and its perspectives (taken from
// the canvases) open on summaries with a roadmap across them; a project on
// it opens its own summary, with its plan and burndown.
//   make e2e   (or: cd scripts/e2e && node programme.mjs)
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
const file = join(dir, "shop.shape");
const web = join(dir, "web.shape"), app = join(dir, "app.shape");
const day = (n) => new Date(Date.now() + n * 864e5).toISOString().slice(0, 10);
writeFileSync(web, `title: Web shop\nperspectives: project\n\n# what we build\nmindmap scope: Web shop [tags: product] {\n  Checkout\n}\n\ngantt plan: Web plan [start: ${day(-10)}] {\n  Build [${day(-10)}, 3w, done: 40%]\n  Web beta [${day(12)}, milestone]\n}\n\nrisks r: Risks {\n  Card fraud [p: 4, i: 5]\n}\n`);
writeFileSync(app, `title: Mobile app\n\ngantt plan: App plan [start: ${day(-5)}] {\n  App build [${day(-5)}, 4w]\n  Store review [${day(23)}, 1w]\n  App launch [${day(30)}, milestone]\n}\n`);
writeFileSync(file, "title: Shop\n");
const server = spawn(join(root, "target/debug/shape"), [file, "--no-open", "--port", String(port)], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1" }, stdio: "ignore" });
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
    await p.waitForSelector(".dge-canvas"); await sleep(800);

    if (scheme === "light") {
      // two boards come in, each as a canvas with a button that opens it
      const chooser = p.waitForEvent("filechooser");
      await p.locator(".dge-canvas").click({ button: "right", position: { x: 600, y: 400 } }); await sleep(200);
      await p.locator(".dge-menu").getByText("Bring in a board…").click();
      await (await chooser).setFiles([web, app]);
      await until(/\ncanvas web-shop: Web shop \{[\s\S]*\ncanvas mobile-app: Mobile app \{/);
      assert.match(saved(), /web-shop-link: Web shop \[pill, canvas: web-shop/);
      assert.match(saved(), /mobile-app-link: Mobile app \[pill, canvas: mobile-app/);
      assert.match(saved(), /\n  mindmap scope: Web shop \[tags: product\] \{\n    Checkout\n  \}/);
      await p.reload({ waitUntil: "domcontentloaded" }); await p.waitForSelector(".dge-canvas"); await sleep(800);
    }

    // its perspectives come from the canvases; Delivery leads with whose milestone it is and the roadmap
    const options = await p.locator(".dge-persp option").allTextContents();
    assert.ok(options.includes("Delivery") && options.includes("Product"), options.join(", "));
    // opening it asks how to view it, as on any board with perspectives
    await p.locator(".dge-persp-card", { hasText: "Delivery" }).click(); await sleep(600);
    assert.equal(await p.locator(".dge-persp").inputValue(), "delivery");
    assert.match(await p.locator('[data-id="@dash/head/:"]').textContent(), /Now delivering: Web shop · Web beta/);
    assert.equal(await p.locator('[data-id="roadmap"]').count() > 0, true);
    assert.equal(await p.locator('[data-id="@dash/rm/mobile-app:plan"]').count() > 0, true);
    // the projects' details are theirs: no burndown here
    assert.equal(await p.locator('[data-id="burndown"]').count(), 0);
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "programme-" + scheme + ".png") });
    // a bar opens that project's own summary: its plan and burndown
    await p.locator('.dg-node[data-id="@dash/rm/mobile-app:plan"]').click(); await sleep(600);
    assert.equal(await p.locator(".dge-crumbs").textContent(), "Shop/Mobile app");
    assert.equal(await p.locator(".dge-persp").inputValue(), "delivery");
    assert.match(await p.locator('[data-id="@dash/head/:"]').textContent(), /Now delivering: App launch/);
    assert.ok(await p.locator('[data-id="burndown"]').count() > 0, "its burndown");
    assert.equal(await p.locator('[data-id="roadmap"]').count(), 0);
    assert.ok(await p.locator('[data-id="gantt"]').count() > 0, "its plan");
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "programme-project-" + scheme + ".png") });
    // its rows open what they stand for on the board, in that canvas (← back to its summary)
    await p.locator('.dg-node[data-id="@dash/ga/:app-build"]').click(); await sleep(600);
    assert.equal(await p.locator(".dge-persp").inputValue(), "");
    assert.equal(await p.locator(".dge-crumbs").textContent(), "Shop/Mobile app");
    await p.locator(".dge-back").click(); await sleep(600);
    assert.equal(await p.locator(".dge-persp").inputValue(), "delivery");
    assert.ok(await p.locator('[data-id="burndown"]').count() > 0, "back on its summary");
    if (scheme === "light") {
      // up to the whole board's summary; a bar's badge says how many risks its canvas holds, and opens its summary
      await p.locator(".dge-crumbs button", { hasText: "Shop" }).click(); await sleep(600);
      const badge = p.locator('.dg-open[data-canvas="web-shop"]');
      assert.match(await badge.locator("title").textContent(), /Open “Web shop”: 1 risk, the worst 20 of 25/);
      await badge.click(); await sleep(600);
      assert.equal(await p.locator(".dge-crumbs").textContent(), "Shop/Web shop");
      assert.ok(await p.locator('[data-id="burndown"]').count() > 0, "the web shop's burndown");
      // on its board, a task gets a canvas of its own, for its risks and the rest of its delivery
      await p.selectOption(".dge-persp", ""); await sleep(600);
      await p.locator('.dg-node[data-id="build"]').click({ button: "right" }); await sleep(200);
      await p.locator(".dge-menu").getByText("Give it a canvas").click(); await sleep(700);
      assert.equal(await p.locator(".dge-crumbs").textContent(), "Shop/Web shop/Build");
      await until(/  Build \[[^\]]*canvas: build-2[^\]]*\]/);
      assert.match(saved(), /\n  canvas build-2: Build \{\n    risks risks: Risks to Build \{\}\n  \}/);
      if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "programme-task-canvas.png") });
    }
    assert.deepEqual(errors, []);
    await p.close();
  }
  await browser.close();
  console.log("programme e2e: ok");
} catch (e) {
  console.error(e);
  failed = true;
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
