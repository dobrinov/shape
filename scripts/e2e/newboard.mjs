// End-to-end test of a new board (`shape` with no file): it opens on a
// draft, offers to save it after a few changes, asks for a name and a
// folder, then whether to always save new boards there; the next new board
// asks only for its name, in that folder.
//   make e2e   (or: cd scripts/e2e && node newboard.mjs)
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const config = join(dir, "config"), drafts = join(config, "drafts"), boards = join(dir, "boards");
mkdirSync(drafts, { recursive: true });
mkdirSync(boards);
const STARTER = "idea: Start here\nidea -> next: then\n";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const until = async (f, what) => { for (let i = 0; i < 40 && !f(); i++) await sleep(100); assert.ok(f(), what); };
// a draft is a board in Shape's drafts folder (what `shape` with no file opens)
function start(name, port) {
  const file = join(drafts, name);
  writeFileSync(file, STARTER);
  const server = spawn(join(root, "target/debug/shape"), [file, "--no-open", "--port", String(port)], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1", SHAPE_CONFIG_DIR: config }, stdio: "ignore" });
  return { file, server };
}
const servers = [];
let failed = false;
try {
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  let k = 0;
  for (const scheme of ["light", "dark"]) {
    const port = 7990 + Math.floor(Math.random() * 100);
    const { file, server } = start(`Untitled-${scheme}.shape`, port);
    servers.push(server);
    await sleep(600);
    const p = await browser.newPage({ viewport: { width: 1300, height: 820 }, colorScheme: scheme });
    const errors = [];
    p.on("pageerror", (e) => errors.push(e.message));
    await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
    await p.waitForSelector(".dge-canvas"); await sleep(600);
    assert.equal(await p.title(), "Untitled — Shape");
    assert.match(await p.locator("body").textContent(), /Draft · not saved yet/);
    // play around: no question yet; after a few changes Shape offers to save it
    const add = (n) => p.evaluate((i) => window.shapeEditor.apply([{ op: "add_node", id: "box" + i, label: "Box " + i }]), n);
    for (let i = 0; i < 4; i++) { await add(k++); await sleep(150); }
    assert.equal(await p.locator(".dge-toast.show .dge-toast-act").count(), 0, "not yet");
    await add(k++); await sleep(300);
    assert.match(await p.locator(".dge-toast.show").textContent(), /not saved anywhere yet/);
    if (scheme === "light") {
      await p.locator(".dge-toast.show .dge-toast-act").click(); await sleep(300);
      // its name and folder (no window here: the folder is typed)
      await p.locator('.dge-dialog input.dge-input').first().fill("Launch plan");
      await p.locator('.dge-dialog input.dge-input').nth(1).fill(boards);
      if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "newboard-save-" + scheme + ".png") });
      await p.locator(".dge-dialog button.primary", { hasText: "Save" }).click(); await sleep(400);
      const to = join(boards, "Launch plan.shape");
      await until(() => existsSync(to), "saved there");
      assert.ok(!existsSync(file), "the draft is gone");
      assert.match(readFileSync(to, "utf8"), /box0: Box 0/);
      // always there?
      assert.match(await p.locator(".dge-dialog").textContent(), /Always save new boards in/);
      await p.locator(".dge-dialog button.primary", { hasText: "Always here" }).click(); await sleep(300);
      assert.equal(JSON.parse(readFileSync(join(config, "prefs.json"), "utf8")).new_boards, boards);
      // from now on it is that file
      assert.equal(await p.title(), "Launch plan.shape — Shape");
      await add(k++); await sleep(800);
      assert.match(readFileSync(to, "utf8"), /box5: Box 5/);
    } else {
      // the next new board: ⌘S asks only for the name, in the folder kept
      await p.locator(".dge-canvas").click({ position: { x: 60, y: 700 } });
      await p.keyboard.press("Meta+s"); await sleep(300);
      assert.equal(await p.locator('.dge-dialog input.dge-input').nth(1).inputValue(), boards, "the folder kept");
      await p.locator('.dge-dialog input.dge-input').first().fill("Launch plan");
      await p.locator(".dge-dialog button.primary", { hasText: "Save" }).click(); await sleep(400);
      assert.match(await p.locator(".dge-dialog .dge-error").textContent(), /is there already/);
      if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "newboard-save-" + scheme + ".png") });
      await p.locator('.dge-dialog input.dge-input').first().fill("Launch plan 2");
      await p.locator(".dge-dialog button.primary", { hasText: "Save" }).click(); await sleep(400);
      await until(() => existsSync(join(boards, "Launch plan 2.shape")), "saved in the folder kept");
      assert.equal(await p.locator(".dge-dialog:visible").count(), 0, "no question this time");
    }
    assert.deepEqual(errors, []);
    await p.close();
  }
  await browser.close();
  console.log("newboard e2e: ok");
} catch (e) {
  console.error(e);
  failed = true;
} finally {
  servers.forEach((s) => s.kill());
  process.exit(failed ? 1 : 0);
}
