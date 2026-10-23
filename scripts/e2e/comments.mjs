// End-to-end test of a comment about several elements: ⇧-click gathers them
// with the comment tool (or right-click a selection → Comment on these…),
// the comment names them all, its thread highlights them, and removing one
// leaves it on the others.
//   make e2e   (or: cd scripts/e2e && node comments.mjs)
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
const port = 7890 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "shop.shape");
writeFileSync(file, "title: Shop\n\napi: Orders API\ndb: Database\nweb: Web app\nweb -> api\napi -> db\n");
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
    await p.waitForSelector(".dge-canvas"); await sleep(600);
    if (scheme === "light") {
      // the comment tool: ⇧-click gathers, a click on one of them comments on all
      await p.locator('.dge-tool[data-tool="comment"]').click(); await sleep(200);
      await p.locator('.dg-node[data-id="db"]').click({ modifiers: ["Shift"] }); await sleep(200);
      await p.locator('.dg-node[data-id="api"]').click({ modifiers: ["Shift"] }); await sleep(200);
      await p.locator('.dg-node[data-id="db"]').click(); await sleep(300);
      assert.match(await p.locator(".dge-bubble textarea").getAttribute("placeholder"), /Comment on Database and Orders API/);
      await p.locator(".dge-bubble textarea").fill("Which one owns the orders?");
      await p.keyboard.press("Meta+Enter"); await sleep(400);
      await until(/\ncomment c1 on db, api[^:]*: Which one owns the orders\?/);
      // right-click a selection → Comment on these…
      await p.locator('.dge-tool[data-tool="select"]').click(); await sleep(100);
      await p.locator('.dg-node[data-id="web"]').click(); await sleep(100);
      await p.locator('.dg-node[data-id="api"]').click({ modifiers: ["Shift"] }); await sleep(100);
      await p.locator('.dg-node[data-id="web"]').click({ button: "right" }); await sleep(200);
      await p.locator(".dge-menu").getByText("Comment on these…").click(); await sleep(200);
      await p.locator(".dge-bubble textarea").fill("Same team?");
      await p.keyboard.press("Meta+Enter"); await sleep(400);
      await until(/\ncomment c2 on web, api[^:]*: Same team\?/);
    }
    // its thread names them and highlights them all
    await p.locator('.dg-pin[data-comment="c1"]').click(); await sleep(300);
    assert.match(await p.locator(".dge-bubble").textContent(), /Comment on Database and Orders API/);
    assert.equal(await p.locator('.dg-node.cmt[data-id="api"]').count() + await p.locator('.dg-node.cmt[data-id="db"]').count() >= 2, true);
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "comments-" + scheme + ".png") });
    if (scheme === "dark") {
      // removing one leaves it on the other
      await p.keyboard.press("Escape"); await sleep(100);
      await p.locator('.dg-node[data-id="db"]').click({ button: "right" }); await sleep(200);
      await p.locator(".dge-menu").getByText("Delete", { exact: true }).click(); await sleep(400);
      await until(/\ncomment c1 on api[^:]*: Which one/);
    }
    assert.deepEqual(errors, []);
    await p.close();
  }
  await browser.close();
  console.log("comments e2e: ok");
} catch (e) {
  console.error(e);
  failed = true;
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
