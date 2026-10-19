// End-to-end test of database schemas: Blocks → Database schema puts a
// structure.sql's tables on the board; when the file changes, opening the
// board offers to update them, and right-click → Update reads it again.
//   make e2e   (or: cd scripts/e2e && node schema.mjs)
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
const port = 7390 + Math.floor(Math.random() * 100);
const dir = mkdtempSync(join(tmpdir(), "shape-e2e-"));
const file = join(dir, "board.shape");
mkdirSync(join(dir, "db"));
const sql = (extra) => `CREATE TABLE public.teams (\n  id bigint NOT NULL,\n  name text NOT NULL\n);\n` +
  `CREATE TABLE public.users (\n  id bigint NOT NULL,\n  team_id bigint,\n  email text NOT NULL${extra}\n);\n` +
  `ALTER TABLE ONLY public.teams ADD CONSTRAINT teams_pkey PRIMARY KEY (id);\n` +
  `ALTER TABLE ONLY public.users ADD CONSTRAINT users_pkey PRIMARY KEY (id);\n` +
  `ALTER TABLE ONLY public.users ADD CONSTRAINT fk_team FOREIGN KEY (team_id) REFERENCES public.teams(id);\n`;
writeFileSync(join(dir, "db", "structure.sql"), sql(""));
writeFileSync(file, "title: App\n\napi: API\n");
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
    await p.waitForSelector(".dge-canvas svg"); await sleep(300);

    if (scheme === "light") {
      // Blocks → Database schema: the project's schema file is offered first
      await p.locator(".dge-blocks-btn").click(); await p.locator('[data-block="schema"]').click(); await sleep(400);
      assert.equal(await p.locator(".dge-modal select").inputValue(), "db/structure.sql");
      if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "schema-dialog.png") });
      await p.locator(".dge-modal button.primary").click();
      await until(/schema db: Database \[file: db\/structure\.sql\] \{\n  entity teams \{\n    id:   bigint \[pk\]\n    name: text\n  \}\n  entity users \{\n    id:      bigint \[pk\]\n    team_id: bigint \[null, fk: teams\]\n    email:   text\n  \}\n\}/);
      assert.equal(await p.locator('.dg-group[data-id="users"]').count() > 0, true);
    }

    // the file changes: opening the board says so, and Update brings it in
    writeFileSync(join(dir, "db", "structure.sql"), sql(scheme === "light" ? ",\n  bio text" : ",\n  bio text,\n  avatar_url text"));
    await p.reload({ waitUntil: "domcontentloaded" }); await p.waitForSelector(".dge-canvas svg");
    await p.waitForSelector(".dge-toast.act", { timeout: 5000 });
    assert.match(await p.locator(".dge-toast").innerText(), /db\/structure\.sql changed \(1 table changed\)/);
    if (process.env.SHOTS) await p.screenshot({ path: join(process.env.SHOTS, "schema-" + scheme + ".png") });
    await p.locator(".dge-toast-act").click();
    await until(scheme === "light" ? /    email:   text\n    bio:     text \[null\]\n  \}/ : /    avatar_url: text \[null\]\n  \}/);

    // right-click the schema → Update from the file; up to date now
    await p.locator('.dg-group[data-id="db"]').first().click({ button: "right", position: { x: 30, y: 12 } });
    await p.getByText("Update from db/structure.sql").click(); await sleep(300);
    assert.match(await p.locator(".dge-toast").innerText(), /up to date/);
    assert.deepEqual(errors, []);
    await p.close();
  }
  await browser.close();
  console.log("schema: ok");
} catch (e) {
  failed = true;
  console.error(e);
} finally {
  server.kill();
  process.exit(failed ? 1 : 0);
}
