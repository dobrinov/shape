// End-to-end test of timelines in a real browser: the Timeline tool, adding
// an event, marking it important, editing as text, a date | event table
// turned into a timeline, and an event's hover card.
//   make e2e   (or: cd scripts/e2e && node timeline.mjs)
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdtempSync, writeFileSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const bin = join(root, "target", "debug", "diagram");
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const port = 6490 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "diagram-e2e-"));
const file = join(dir, "board.diagram");
writeFileSync(file, "title: History\n\ntable log: Log [x: 0, y: 400] {\n  date | event\n  Dec 2024 | Deal signed\n  7–14 May 2026 | Visa approves\n}\n");
const server = spawn(bin, [file, "--no-open", "--port", String(port)], {
  env: { ...process.env, DIAGRAM_WEB_DIR: join(root, "web"), DIAGRAM_NO_UPDATE_CHECK: "1" },
  stdio: "ignore",
});
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
const MOD = process.platform === "darwin" ? "Meta" : "Control";
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  const p = await browser.newPage({ viewport: { width: 1300, height: 800 } });
  const errors = [];
  p.on("pageerror", (e) => errors.push(e.message));
  await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
  await p.waitForSelector(".dge-canvas svg"); await sleep(300);

  // the table becomes a timeline: ids follow the labels, nothing else changes
  const table = await p.locator('.dg-group[data-id="log"]').first().boundingBox();
  await p.mouse.click(table.x + 20, table.y + 8, { button: "right" });
  await p.locator(".dge-menu button", { hasText: "Show as timeline" }).click(); await sleep(700);
  assert.match(saved(), /timeline log: Log \[x: 0, y: 400\] \{\n  Dec 2024: Deal signed\n  7–14 May 2026: Visa approves\n\}/);

  // the Timeline tool, then an event added from its menu, then starred
  await p.locator('.dge-tool[data-act="timeline"]').click(); await sleep(300);
  await p.keyboard.type("Launch"); await p.keyboard.press("Enter"); await sleep(600);
  assert.match(saved(), /timeline launch: Launch \{\n  \d{4}-\d\d-\d\d: Today \[important\]\n/);
  const tl = await p.locator('.dg-group[data-id="launch"]').first().boundingBox();
  await p.mouse.click(tl.x + 20, tl.y + 8, { button: "right" });
  await p.locator(".dge-menu button", { hasText: "Add event…" }).click();
  await p.keyboard.type("28 Oct 2026: Open beta"); await p.keyboard.press("Enter"); await sleep(600);
  assert.match(saved(), /  28 Oct 2026: Open beta\n/);
  await p.locator('.dg-node[data-id="launch.open-beta"]').click({ button: "right" });
  await p.locator(".dge-menu button", { hasText: "Mark important" }).click(); await sleep(600);
  assert.match(saved(), /  28 Oct 2026: Open beta \[important\]\n/);

  // a date that cannot be read is refused, and says why
  await p.locator('.dg-node[data-id="launch.open-beta"]').click({ button: "right" });
  await p.locator(".dge-menu button", { hasText: "Change date…" }).click();
  await p.keyboard.press(MOD + "+a"); await p.keyboard.type("someday"); await p.keyboard.press("Enter"); await sleep(400);
  assert.match(await p.locator(".dge-toast").textContent(), /cannot read the date `someday`/);
  assert.match(saved(), /  28 Oct 2026: Open beta \[important\]\n/);

  // edit as text: lines become events, matched by position
  await p.mouse.click(tl.x + 20, tl.y + 8, { button: "right" });
  await p.locator(".dge-menu button", { hasText: "Edit as text…" }).click();
  await p.locator(".dge-modal textarea").fill("Q1 2026: Kick-off\n28 Oct 2026: Open beta [important]\nmid Dec 2026: Spend Cards beta");
  await p.locator(".dge-modal button.primary").click(); await sleep(700);
  assert.match(saved(), /timeline launch: Launch \{\n  Q1 2026: Kick-off\n  28 Oct 2026: Open beta \[important\]\n  mid Dec 2026: Spend Cards beta\n\}/);

  // an event with a detail shows it on hover
  await p.evaluate(() => window.diagramEditor.apply([{ op: "update_node", id: "launch.open-beta", detail: "Virtual cards for everyone" }])); await sleep(500);
  await p.mouse.move(5, 5);
  await p.locator('.dg-node[data-id="launch.open-beta"]').hover(); await sleep(800);
  assert.match(await p.locator(".dge-tip").textContent(), /Open beta.*Virtual cards for everyone/s);
  await p.screenshot({ path: join(dir, "timeline.png") });
  assert.deepEqual(errors, []);
  await browser.close();
  console.log("ok — timeline e2e passed (screenshot: " + join(dir, "timeline.png") + ")");
} catch (e) {
  failed = true;
  console.error(e.message || e);
  console.error("\nfile:\n" + saved());
} finally {
  server.kill();
}
process.exit(failed ? 1 : 0);
