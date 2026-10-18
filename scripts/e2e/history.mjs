// End-to-end test of a board's history when git does not track it: the
// editor asks who you are (git does not know), a kept version is brought
// back from the History panel, and the save is kept signed with your name.
//   make e2e   (or: cd scripts/e2e && node history.mjs)
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, readdirSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const bin = join(root, "target", "debug", "shape");
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const port = 6790 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "diagram-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: History\n\na: Alpha\n");
const kept = join(dir, ".shape-history", "board.shape");
mkdirSync(kept, { recursive: true });
writeFileSync(join(kept, "1700000000__Ana+Bell.shape"), "title: History\n\na: Old Alpha\n");
// git without a name, so the editor has to ask
const noname = join(dir, "gitconfig");
writeFileSync(noname, "");
const server = spawn(bin, [file, "--no-open", "--port", String(port)], {
  env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1", GIT_CONFIG_GLOBAL: noname, GIT_CONFIG_NOSYSTEM: "1" },
  stdio: "ignore",
});
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  // one browser profile, so the name is remembered
  const ctx = await browser.newContext({ viewport: { width: 1300, height: 800 } });
  for (const scheme of ["light", "dark"]) {
    const p = await ctx.newPage();
    await p.emulateMedia({ colorScheme: scheme });
    const errors = [];
    p.on("pageerror", (e) => errors.push(e.message));
    await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
    await p.waitForSelector(".dge-canvas svg"); await sleep(600);
    if (scheme === "light") {
      // who are you?
      await p.waitForSelector(".dge-name-input");
      if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "name.png") });
      await p.locator(".dge-name-input").fill("Bo Chen");
      await p.keyboard.press("Enter"); await sleep(300);
    } else {
      assert.equal(await p.locator(".dge-name-input").count(), 0, "the name is remembered");
    }
    await p.locator('[data-act="history"]').click(); await sleep(500);
    const rows = p.locator(".dge-history-row");
    if (scheme === "light") {
      assert.equal(await rows.count(), 1);
      assert.match(await rows.first().innerText(), /Ana Bell/);
      // the exact time on hover
      assert.match(await rows.first().locator(".dge-history-text").getAttribute("title"), /2023/);
      await rows.first().getByRole("button", { name: "Restore" }).click(); await sleep(1200);
      assert.match(saved(), /a: Old Alpha/);
      const names = readdirSync(kept);
      assert.ok(names.some((n) => n.endsWith("__Bo+Chen.shape")), names.join(", "));
    } else {
      assert.equal(await rows.count(), 2);
      assert.match(await rows.first().innerText(), /Bo Chen/);
      if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "history-dark.png") });
      // compare with Ana's version
      await rows.nth(1).getByRole("button", { name: "Changes" }).click(); await sleep(500);
    }
    if (scheme === "light" && process.env.SHOTS) {
      await p.locator('[data-act="history"]').click(); await sleep(500);
      await p.screenshot({ path: join(process.env.SHOTS, "history-light.png") });
    }
    assert.deepEqual(errors, []);
    await p.close();
  }
  await browser.close();
  console.log("history: ok");
} catch (e) {
  failed = true;
  console.error(e);
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
