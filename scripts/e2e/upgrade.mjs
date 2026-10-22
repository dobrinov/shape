// End-to-end test of Rethink after an upgrade: a board on built-in
// perspectives that this Shape has not rethought is offered a Rethink once;
// the Rethink is told what is new, and keeping it records the version.
//   make e2e   (or: cd scripts/e2e && node upgrade.mjs)
import { chromium } from "playwright";
import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const bin = join(root, "target/debug/shape");
const version = execFileSync(bin, ["--version"], { encoding: "utf8" }).trim().split(" ").pop();
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const port = 7790 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "shop.shape");
writeFileSync(file, "title: Shop\nperspectives: project\nrethought: 0.70.0\n\napi: Orders API\ndb: Postgres [cylinder]\napi -> db\n");
const server = spawn(bin, [file, "--no-open", "--port", String(port), "--ai", join(here, "fake-ops.sh")], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
const until = async (re) => { for (let i = 0; i < 40 && !re.test(saved()); i++) await sleep(100); assert.match(saved(), re); };
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  const ctx = await browser.newContext({ viewport: { width: 1300, height: 820 } });
  const p = await ctx.newPage();
  const errors = [];
  p.on("pageerror", (e) => errors.push(e.message));
  await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
  await p.waitForSelector(".dge-persp-card"); await sleep(300);
  await sleep(1700);
  assert.equal(await p.locator(".dge-toast.show.act").count(), 0, "not while the board asks how to view it");
  await p.locator(".dge-persp-card", { hasText: "Full board" }).click();
  // what is new since 0.70 for a project team's perspectives, with a Rethink to bring it in
  await p.waitForSelector(".dge-toast.show.act", { timeout: 5000 });
  const toast = await p.locator(".dge-toast").innerText();
  assert.match(toast, /can do more with this board: .*test scenarios for QA/);
  assert.doesNotMatch(toast, /for Users/, "a project board has no Users perspective");
  await p.locator(".dge-toast-act").click();
  await p.waitForSelector(".dge-preview-bar");
  assert.match(await p.locator(".dge-preview-bar").innerText(), /with what Shape can do now/);
  await p.locator(".dge-preview-bar button.primary").click();
  await until(new RegExp("\\nrethought: " + version.replace(/\./g, "\\.") + "\\n"));
  assert.match(saved(), /group storage: Storage/);
  // up to date: no offer again
  await p.reload({ waitUntil: "domcontentloaded" });
  await p.waitForSelector(".dge-canvas"); await sleep(400);
  if (await p.locator(".dge-persp-card").count()) await p.locator(".dge-persp-card", { hasText: "Full board" }).click();
  await sleep(2200);
  assert.equal(await p.locator(".dge-toast.show.act").count(), 0);
  assert.deepEqual(errors, []);
  await browser.close();
  console.log("upgrade e2e: ok");
} catch (e) {
  console.error(e);
  failed = true;
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
