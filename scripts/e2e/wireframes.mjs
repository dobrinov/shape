// End-to-end test of wireframes: a phone screen from the Blocks palette,
// rewritten as text, a web page, and a flow from a button to the page.
//   make e2e   (or: cd scripts/e2e && node wireframes.mjs)
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
const port = 7290 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "diagram-e2e-"));
const file = join(dir, "board.shape");
writeFileSync(file, "title: App\n");
const server = spawn(join(root, "target/debug/shape"), [file, "--no-open", "--port", String(port)], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const saved = () => readFileSync(file, "utf8");
const until = async (re) => { for (let i = 0; i < 40 && !re.test(saved()); i++) await sleep(100); assert.match(saved(), re); };
let failed = false;
try {
  await sleep(600);
  const browser = await chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {});
  const p = await browser.newPage({ viewport: { width: 1400, height: 900 } });
  const errors = [];
  p.on("pageerror", (e) => errors.push(e.message));
  await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
  await p.waitForSelector(".dge-canvas svg"); await sleep(300);

  // a phone screen from the palette, named
  await p.locator(".dge-blocks-btn").click(); await p.locator('[data-block="screen"]').click(); await sleep(300);
  await p.keyboard.type("Sign in"); await p.keyboard.press("Enter");
  await until(/screen sign-in: Sign in \[mobile\] \{\n  header: App\n/);

  // rewritten as text
  await p.locator('.dg-group[data-id="sign-in"]').first().click({ button: "right", position: { x: 30, y: 12 } });
  await p.getByText("Edit as text…").click(); await sleep(200);
  await p.locator(".dge-modal textarea").fill("header: Acme\ntitle: Welcome back\ninput email: Email\npassword: Password\nbutton submit: Sign in [blue]\nlink forgot: Forgot password?");
  await p.locator(".dge-modal button.primary").click();
  await until(/screen sign-in: Sign in \[mobile\] \{\n  header: Acme\n  title: Welcome back\n  input email: Email\n  password: Password\n  button submit: Sign in \[blue\]\n  link forgot: Forgot password\?\n\}/);
  assert.equal(await p.locator('.dg-node[data-id="sign-in.submit"]').count(), 1);

  // the component bar: select a component, add a button after it, type its text
  await p.locator('.dg-node[data-id="sign-in.password"]').click(); await sleep(200);
  assert.ok(await p.locator(".dge-screenbar").isVisible());
  await p.locator('.dge-screenbar [data-widget="button"]').click(); await sleep(300);
  await p.keyboard.type("Show password"); await p.keyboard.press("Enter");
  await until(/  password: Password\n  button show-password: Show password\n  button submit: Sign in \[blue\]/);
  // ⌥↑ moves it up
  await p.locator('.dg-node[data-id="sign-in.show-password"]').click(); await sleep(200);
  await p.keyboard.press("Alt+ArrowUp");
  await until(/  input email: Email\n  button show-password: Show password\n  password: Password/);
  // every component from the menu
  await p.locator('.dge-screenbar [data-act="widgets"]').click(); await sleep(200);
  assert.ok((await p.locator(".dge-widgets [data-widget]").count()) >= 40);
  await p.locator('.dge-widgets [data-widget="toggle"]').click(); await sleep(300);
  await p.keyboard.press("Enter");
  await until(/  button show-password: Show password\n  toggle: Setting\n/);
  if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "wire-bar.png") });
  // back to the text the rest of the test expects
  await p.locator('.dg-group[data-id="sign-in"]').first().click({ button: "right", position: { x: 30, y: 12 } });
  await p.getByText("Edit as text…").click(); await sleep(200);
  await p.locator(".dge-modal textarea").fill("header: Acme\ntitle: Welcome back\ninput email: Email\npassword: Password\nbutton submit: Sign in [blue]\nlink forgot: Forgot password?");
  await p.locator(".dge-modal button.primary").click();
  await until(/password: Password\n  button submit/);

  // a web page, and a flow from the button to it
  await p.locator(".dge-blocks-btn").click(); await p.locator('[data-block="page"]').click(); await sleep(300);
  await p.keyboard.type("Home"); await p.keyboard.press("Enter");
  await until(/screen home: Home \[web\] \{\n  navbar: App \| Product \| Pricing \| Sign in\n/);
  // the button leads to the page: an arrow dropped anywhere on it
  await p.keyboard.press("f"); await sleep(300);
  const a = await p.locator('.dg-node[data-id="sign-in.submit"]').boundingBox();
  const b = await p.locator('.dg-group[data-id="home"]').first().boundingBox();
  await p.locator(".dge-canvas").click({ position: { x: 5, y: 400 } });
  await p.keyboard.press("x");
  await p.mouse.move(a.x + a.width / 2, a.y + a.height / 2); await p.mouse.down();
  await p.mouse.move(b.x + b.width / 2, b.y + b.height / 2, { steps: 8 }); await p.mouse.up();
  await until(/sign-in\.submit -> home/);
  // from the forgot link onto empty canvas: the next screen, a phone too
  const f = await p.locator('.dg-node[data-id="sign-in.forgot"]').boundingBox();
  await p.keyboard.press("x");
  await p.mouse.move(f.x + f.width / 2, f.y + f.height / 2); await p.mouse.down();
  await p.mouse.move(f.x + 40, 860, { steps: 8 }); await p.mouse.up();
  await sleep(300);
  await p.keyboard.type("Reset password"); await p.keyboard.press("Enter");
  await until(/screen reset-password: Reset password \[mobile, x: -?\d+, y: -?\d+\] \{/);
  await until(/sign-in\.forgot -> reset-password/);
  if (process.env.SHOTS) {
    await p.keyboard.press("Escape"); await p.keyboard.press("f"); await sleep(300);
    await p.screenshot({ path: join(process.env.SHOTS, "wire-light.png") });
    await p.locator('button[title="Light / dark"]').click(); await sleep(300);
    await p.screenshot({ path: join(process.env.SHOTS, "wire-dark.png") });
  }
  // a ready-made flow: three screens, linked
  await p.keyboard.press("Escape");
  await p.locator(".dge-blocks-btn").click(); await p.locator('[data-block="flow"]').click(); await sleep(600);
  await until(/screen home-2: Home \[mobile\][\s\S]*sign-in-2\.go -> home-2: signs in\nhome-2\.items -> detail: opens/);
  assert.deepEqual(errors, []);
  await browser.close();
  console.log("wireframes: ok");
} catch (e) {
  failed = true;
  console.error(e);
  console.error("\nfile:\n" + saved());
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
