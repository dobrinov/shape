// End-to-end test of gantt groups: a project's sub-projects and their phases
// are indented under them; a group folds (on the board and, folded by default,
// on the summary's roadmap); a bar's start is dragged; a group moves with its
// parts; a task goes inside the one above it from its menu.
//   make e2e   (or: cd scripts/e2e && node groups.mjs)
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
const port = 7790 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "shop.shape");
const day = (n) => new Date(Date.now() + n * 864e5).toISOString().slice(0, 10);
writeFileSync(file, `title: Shop
perspectives: project

web_link: Web shop [pill, canvas: web]
app_link: Mobile app [pill, canvas: app]

canvas web: Web shop {
  gantt plan: Web plan [start: ${day(-30)}] {
    Storefront
      Spike [${day(-30)}, 2w, done]
      Build [4w, done: 40%]
      Beta [${day(20)}, milestone]
    Checkout [collapsed]
      Spike [${day(-10)}, 1w, done]
      Store approval [${day(-3)}, 5w]
      Beta [${day(40)}, milestone]
    Polish [1w]
  }
}

canvas app: Mobile app {
  gantt plan: App plan [start: ${day(-5)}] {
    App build [${day(-5)}, 4w]
    Store review [${day(23)}, 1w]
    App launch [${day(30)}, milestone]
  }
}
`);
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
    await p.waitForSelector(".dge-persp-card"); await sleep(400);

    // the roadmap: a row per project, its plan folded into it
    await p.locator(".dge-persp-card", { hasText: "Delivery" }).click(); await sleep(600);
    assert.equal(await p.locator('.dg-node[data-id="@dash/rm/web:storefront"]').count(), 0, "folded");
    await p.locator('.dg-gfold[data-node="@dash/rm/web:plan"]').click(); await sleep(500);
    // unfolded: its sub-projects, each folded
    assert.ok(await p.locator('.dg-node[data-id="@dash/rm/web:storefront"]').count() > 0, "its sub-projects");
    assert.equal(await p.locator('.dg-node[data-id="@dash/rm/web:storefront-build"]').count(), 0);
    await p.locator('.dg-gfold[data-node="@dash/rm/web:storefront"]').click(); await sleep(500);
    assert.ok(await p.locator('.dg-node[data-id="@dash/rm/web:storefront-build"]').count() > 0, "its phases");
    // the folded Checkout shows its beta on its row
    assert.ok(await p.locator('.dg-node[data-id="@dash/rm/web:checkout-beta"]').count() > 0);
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "groups-roadmap-" + scheme + ".png") });

    if (scheme === "light") {
      // on the board, in the project's canvas
      await p.selectOption(".dge-persp", ""); await sleep(400);
      await p.locator('button[title="Fit (F)"]').click(); await sleep(300);
      await p.locator('.dg-node[data-id="web_link"]').dblclick(); await sleep(700);
      assert.equal(await p.locator(".dge-crumbs").textContent(), "Shop/Web shop");
      // Checkout unfolds from its mark
      assert.equal(await p.locator('.dg-node[data-id="checkout-store-approval"]').count(), 0);
      await p.locator('.dg-gfold[data-node="checkout"]').click(); await sleep(500);
      await until(/\n {4}Checkout\n {6}Spike/);
      // the start of a bar is dragged later: it starts later, it ends where it did
      const bar = await p.locator('.dg-node[data-id="checkout-store-approval"] .dg-shape').boundingBox();
      const dayPx = bar.width / 35;
      await p.mouse.move(bar.x + 2, bar.y + bar.height / 2);
      await p.mouse.down();
      await p.mouse.move(bar.x + 2 + dayPx * 3, bar.y + bar.height / 2, { steps: 6 });
      await p.mouse.move(bar.x + 2 + dayPx * 7, bar.y + bar.height / 2, { steps: 6 });
      await p.mouse.up(); await sleep(400);
      await until(new RegExp(`Store approval \\[${day(4)}, 4w\\]`));
      // a group moves with its parts
      await p.locator('button[title="Fit (F)"]').click(); await sleep(300);
      const grp = await p.locator('.dg-node[data-id="checkout"] .dg-shape').boundingBox();
      await p.mouse.move(grp.x + grp.width / 2, grp.y + 4);
      await p.mouse.down();
      await p.mouse.move(grp.x + grp.width / 2 + dayPx * 3, grp.y + 4, { steps: 6 });
      await p.mouse.move(grp.x + grp.width / 2 + dayPx * 7, grp.y + 4, { steps: 6 });
      await p.mouse.up(); await sleep(400);
      await until(new RegExp(`Spike \\[${day(-3)}, 1w, done\\][\\s\\S]*Store approval \\[${day(11)}, 4w\\][\\s\\S]*Beta \\[${day(47)}, milestone\\]`));
      // Polish goes inside Checkout, from its menu
      await p.locator('.dg-node[data-id="polish"]').click({ button: "right" }); await sleep(200);
      await p.locator(".dge-menu").getByText("Part of “Checkout”").click();
      await until(/\n {6}Polish \[id: polish, 1w\]\n {2}\}/);
      if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "groups-board.png") });
    }
    assert.deepEqual(errors, []);
    await p.close();
  }
  await browser.close();
  console.log("groups e2e: ok");
} catch (e) {
  console.error(e);
  failed = true;
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
