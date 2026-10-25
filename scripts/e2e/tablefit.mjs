// End-to-end test that table text stays in its cells in a real browser
// (its font may draw wider than Shape measures): no cell's text runs into
// the next column or out of the table, in light and dark.
//   make e2e   (or: cd scripts/e2e && node tablefit.mjs)
import { chromium, webkit } from "playwright";
import { spawn } from "node:child_process";
import { mkdtempSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const chrome = process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const port = 7130 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "t.shape");
writeFileSync(file, `title: Shop
perspectives: product

table open: Decisions needed {
  decision | where
  Final order types & required info/docs | Items needing business confirmation (§8)\\n› Orders
  Customer-facing status model & wording | Items needing business confirmation (§8)\\n› Orders
  Maker-checker / owner approval reuse | Items needing business confirmation (§8)\\n› Orders
  When is "customer action required" set & resolved | Items needing business confirmation (§8)\\n› Orders
  Signal data model (own object vs tagged sub-type) | Items needing business confirmation (§8)\\n› Orders
}

orders_link: Orders [canvas: orders]

canvas orders: Orders {
  group confirm: Items needing business confirmation (§8) {
    adr d1: Final order types & required info/docs [proposed] {
      context: x
    }
    adr d2: Customer-facing status model & wording [proposed] {
      context: x
    }
    adr d3: Maker-checker / owner approval reuse [proposed] {
      context: x
    }
    adr d4: When is "customer action required" set & resolved [proposed] {
      context: x
    }
    adr d5: Signal data model (own object vs tagged sub-type) [proposed] {
      context: x
    }
  }
}
`);
const server = spawn(join(root, "target/debug/shape"), [file, "--no-open", "--port", String(port)], { env: { ...process.env, SHAPE_WEB_DIR: join(root, "web"), SHAPE_NO_UPDATE_CHECK: "1" }, stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let failed = false;
try {
  await sleep(600);
  // Shape's own window on macOS draws with WebKit (Safari's engine): both, when it is there
  const engines = [["chromium", () => chromium.launch(existsSync(chrome) ? { executablePath: chrome } : {})]];
  if (existsSync(webkit.executablePath())) engines.push(["webkit", () => webkit.launch()]);
  for (const [engine, launch] of engines) {
  const browser = await launch();
  for (const scheme of ["light", "dark"]) {
    const p = await browser.newPage({ viewport: { width: 1300, height: 820 }, colorScheme: scheme });
    await p.goto(`http://127.0.0.1:${port}/`, { waitUntil: "domcontentloaded" });
    await p.waitForSelector(".dge-canvas svg"); await sleep(600);
    // every line of text against the column lines around it (in the drawing's own units)
    const bad = await p.evaluate(() => {
      const t = document.querySelector('.dg-group[data-id="open"]') || document.querySelector('[data-group="open"]');
      const box = t.closest("g") || t;
      const svg = document.querySelector(".dge-canvas svg");
      const ctm = (el) => el.getScreenCTM();
      const xs = [...svg.querySelectorAll('[data-group="open"] line, .dg-chart line')].filter((l) => l.getAttribute("x1") === l.getAttribute("x2")).map((l) => l.getBoundingClientRect().left);
      const out = [];
      const texts = [...svg.querySelectorAll('[data-group="open"] text, .dg-chart text')];
      const frame = (svg.querySelector('[data-group="open"]') || box).getBoundingClientRect();
      for (const el of texts) {
        const r = el.getBoundingClientRect();
        const next = xs.filter((x) => x > r.left + 2).sort((a, b) => a - b)[0] ?? frame.right;
        if (r.right > next + 1) out.push(el.textContent + " ends " + Math.round(r.right - next) + "px past its column");
      }
      return { out, n: texts.length, cols: xs.length };
    });
    assert.ok(bad.n > 10, "the table's text was found (" + bad.n + ")");
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "tablefit-" + engine + "-" + scheme + ".png") });
    assert.deepEqual(bad.out, []);
    // and a summary's table (Product: the decisions needed)
    await p.selectOption(".dge-persp", "product"); await sleep(700);
    const bad2 = await p.evaluate(() => {
      const g = document.querySelector('.dg-group[data-id="decisions"]');
      const svg = document.querySelector(".dge-canvas svg");
      const part = svg.querySelector('[data-group="decisions"]') || g;
      const frame = g.getBoundingClientRect();
      const xs = [...part.querySelectorAll("line")].filter((l) => l.getAttribute("x1") === l.getAttribute("x2")).map((l) => l.getBoundingClientRect().left);
      const texts = [...svg.querySelectorAll("text")].filter((t) => { const r = t.getBoundingClientRect(); return r.top > frame.top + 30 && r.bottom < frame.bottom && r.left > frame.left; });
      const out = [];
      for (const el of texts) {
        const r = el.getBoundingClientRect();
        const next = xs.filter((x) => x > r.left + 2).sort((a, b) => a - b)[0] ?? frame.right - 10;
        if (r.right > next + 1) out.push(el.textContent + " ends " + Math.round(r.right - next) + "px past its column");
      }
      return { out, n: texts.length };
    });
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "tablefit-summary-" + engine + "-" + scheme + ".png") });
    assert.ok(bad2.n > 5, "the summary's table (" + bad2.n + ")");
    assert.deepEqual(bad2.out, []);
    await p.close();
  }
  await browser.close();
  }
  console.log("tablefit e2e: ok (" + engines.map((e) => e[0]).join(", ") + ")");
} catch (e) {
  console.error(e);
  failed = true;
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
