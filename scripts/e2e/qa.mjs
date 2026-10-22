// End-to-end test of QA: ✦ Draft test scenarios puts the AI's scenarios in a
// canvas of their own (more of them the second time, in the same table), a
// scenario is marked failed from its menu, and the QA perspective opens on
// how it works, the scenarios and what has none yet.
//   make e2e   (or: cd scripts/e2e && node qa.mjs)
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
const port = 7690 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "shop.shape");
writeFileSync(file, `title: Web shop
perspectives: product

lanes checkout: Checkout {
  lane shopper: Shopper {
    cart: Open the cart
    pay: Pay
  }
}
cart -> pay

sequence refund: Refund {
  agent: Support agent
  api: Orders API
  agent -> api: refund order
}

screen login: Sign in [web] {
  input email: Email
  button go: Sign in [blue]
}
`);
const server = spawn(join(root, "target/debug/shape"), [file, "--no-open", "--port", String(port), "--ai", join(here, "fake-ops.sh")], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
const until = async (re) => { for (let i = 0; i < 40 && !re.test(saved()); i++) await sleep(100); assert.match(saved(), re); };
const draft = async (p, fab) => {
  if (fab) {
    await p.locator(".dge-ai-fab").click(); await sleep(200);
    await p.locator(".dge-ai-item", { hasText: "Draft test scenarios" }).click();
  } else {
    await p.locator(".dge-canvas").click({ button: "right", position: { x: 40, y: 700 } }); await sleep(200);
    await p.locator(".dge-menu").getByText("✦ Draft test scenarios").click();
  }
  await p.waitForSelector(".dge-preview-bar");
  await p.locator(".dge-preview-bar button.primary").click(); await sleep(300);
};
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  for (const scheme of ["light", "dark"]) {
    const p = await browser.newPage({ viewport: { width: 1300, height: 820 }, colorScheme: scheme });
    const errors = [];
    p.on("pageerror", (e) => errors.push(e.message));
    await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
    await p.waitForSelector(".dge-persp-card"); await sleep(400);
    // QA is one of the product team's perspectives
    assert.match(await p.locator(".dge-persp-card", { hasText: "QA" }).innerText(), /How it works, and the scenarios that test it/);

    if (scheme === "light") {
      await p.locator(".dge-persp-card", { hasText: "QA" }).click(); await sleep(500);
      assert.match(await p.locator('[data-id="@dash/head/:"]').textContent(), /How it works: 3 parts to test/);
      assert.ok(await p.locator('[data-id="howitworks"]').count() > 0, "how it works, as on the board");
      // from the summary (✦), the scenarios are drafted on the board, in a canvas of their own
      await draft(p, true);
      await until(/\ntests-link: Test scenarios \[pill, canvas: tests, tags: qa\]/);
      assert.match(saved(), /\ncanvas tests: Test scenarios \{\n {2}table scenarios: Test scenarios \{\n {4}scenario +\| steps +\| expected +\| covers +\| status\n {4}Pay with a saved card +\| 1\. Open the cart\\n2\. Pay/);
      // again, from the board's menu: one more, in the same table
      await draft(p, false);
      await until(/\n {4}Pay twice +\|/);
      assert.equal(saved().match(/Pay with a saved card/g).length, 1);
      // a scenario is marked from its menu, in its canvas
      await p.locator('button[title="Fit (F)"]').click(); await sleep(300);
      await p.locator('.dg-node[data-id="tests-link"]').dblclick(); await sleep(600);
      assert.equal(await p.locator(".dge-crumbs").textContent(), "Web shop/Test scenarios");
      await p.locator('.dg-node[data-id="scenarios.wrong-password"]').click({ button: "right" }); await sleep(200);
      await p.locator(".dge-menu").getByText("Failed", { exact: true }).click();
      await until(/\n {4}Wrong password +\| 1\. Sign in with a wrong password +\| Says it is wrong +\| Sign in +\| fail +\[red\]/);
      await p.reload({ waitUntil: "domcontentloaded" });
      await p.waitForSelector(".dge-canvas"); await sleep(600);
    }

    // (a fresh page asks how to view the board first)
    if (await p.locator(".dge-persp-card").count()) await p.locator(".dge-persp-card", { hasText: "QA" }).click();
    else await p.selectOption(".dge-persp", "qa");
    await sleep(500);
    assert.match(await p.locator('[data-id="@dash/head/:"]').textContent(), /Tests: 3 scenarios · 1 fail · 2 to run/);
    assert.match(await p.locator('[data-id="@dash/head/:"]').textContent(), /Not covered yet: Refund/);
    assert.match(await p.locator('[data-id="scenarios"]').first().textContent(), /Test scenarios \(3\)/);
    assert.ok(await p.locator('[data-id="@dash/un/:refund"]').count() > 0, "the refund has no scenario yet");
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "qa-" + scheme + ".png") });
    // a scenario's row opens it, in its canvas
    await p.locator('.dg-node[data-id="@dash/qa/tests:scenarios.wrong-password"]').click(); await sleep(600);
    assert.equal(await p.locator(".dge-crumbs").textContent(), "Web shop/Test scenarios");
    assert.deepEqual(errors, []);
    await p.close();
  }
  await browser.close();
  console.log("qa e2e: ok");
} catch (e) {
  console.error(e);
  failed = true;
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
