// End-to-end test of the session log: a session's AI request (prompt, reply,
// time), what was kept, the saves and what they changed, and (with --debug)
// the ops and perspectives, as `shape logs` tells them.
//   make e2e   (or: cd scripts/e2e && node logs.mjs)
import { chromium } from "playwright";
import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync, readFileSync, readdirSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const bin = join(root, "target/debug/shape");
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const port = 7590 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const logs = join(dir, "logs");
const file = join(dir, "shop.shape");
writeFileSync(file, "title: Shop\n\napi: Orders API\ndb: Postgres [cylinder]\napi -> db\n");
const env = { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1", SHAPE_LOG_DIR: logs };
delete env.SHAPE_NO_LOG;
const server = spawn(bin, [file, "--no-open", "--port", String(port), "--ai", join(here, "fake-ops.sh"), "--debug"], { env, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
const shape = (...args) => execFileSync(bin, args, { env, encoding: "utf8" });
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  const p = await browser.newPage({ viewport: { width: 1300, height: 820 } });
  await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
  await p.waitForSelector(".dge-canvas svg"); await sleep(300);

  // a Rethink, kept
  await p.locator(".dge-canvas").click({ position: { x: 20, y: 300 }, button: "right" });
  await p.locator(".dge-menu button, .dge-menu [role=menuitem]", { hasText: "Rethink the board" }).first().click();
  await p.waitForSelector(".dge-preview-bar");
  await p.locator(".dge-preview-bar button.primary").click();
  for (let i = 0; i < 40 && !/group storage/.test(saved()); i++) await sleep(100);
  // a box of one's own: with --debug, its op is logged
  await p.locator(".dge-canvas").dblclick({ position: { x: 900, y: 600 } }); await sleep(200);
  await p.keyboard.type("Cache"); await p.keyboard.press("Enter");
  for (let i = 0; i < 40 && !/Cache/.test(saved()); i++) await sleep(100);
  // an error in the editor is in the log
  await p.evaluate(() => window.dispatchEvent(new ErrorEvent("error", { message: "boom in the test", filename: "http://x/editor.js", lineno: 7 })));
  await sleep(500);
  await browser.close();

  assert.equal(readdirSync(logs).filter((f) => f.endsWith(".jsonl")).length, 1, "one session, one file");
  const out = shape("logs");
  assert.match(out, /started Shape [\d.]+ on .*shop\.shape/);
  assert.match(out, /ai-action: \{"action":"rethink"/);
  assert.match(out, /AI answered in [\d.]+s via `.*fake-ops\.sh`/);
  assert.match(out, /prompt: [\s\S]*# Rethink the board/);
  assert.match(out, /reply: {2}```json[\s\S]*Put the storage in its own section/);
  assert.match(out, /ai-result: \{"kept":true\}/);
  assert.match(out, /saved \(\d+ bytes\)[\s\S]*storage/);
  assert.match(out, /op: \{"ops":"\[\{\\"op\\":\\"add_node\\"/, "--debug logs the ops");
  assert.match(out, /saved \(\d+ bytes\)[\s\S]*Cache/);
  assert.match(out, /error: \{"message":"boom in the test","where":"editor\.js:7"\}/);
  // the AI part alone, the errors alone, the sessions
  const ai = shape("logs", "ai");
  assert.match(ai, /AI answered/);
  assert.doesNotMatch(ai, /saved \(/);
  assert.match(shape("logs", "errors"), /boom in the test/);
  assert.match(shape("logs", "--list"), / 1 {2}\d{4}-\d\d-\d\dT[\d:]+Z {2}.*shop\.shape {2}\(\d+ lines, 1 AI requests, 1 errors\)/);
  // and they can go
  execFileSync(bin, ["logs", "--clear"], { env, stdio: "ignore" });
  assert.equal(readdirSync(logs).filter((f) => f.endsWith(".jsonl")).length, 0);
  console.log("logs e2e: ok");
} catch (e) {
  console.error(e);
  failed = true;
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
