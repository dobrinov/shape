---
name: shape-draw
description: Draw a diagram (architecture, flow, sequence of steps, org or idea board) as a .shape file with the `shape` CLI, render it to PNG/SVG and open it in the editor. Use when the user asks to sketch, draw, visualise or diagram something, or wants a picture of how parts of a codebase fit together.
---

# shape-draw

Diagrams are plain-text `.shape` files. People edit them in a FigJam-like browser editor (`shape FILE`), and they diff cleanly in git.

## Steps

1. If you are not sure of the syntax, read the language reference with `shape spec`. The essentials:
   ```shape
   title: Checkout
   direction: right                    # right | down | left | up
   user: Customer [person]
   group backend: Backend [blue] {
     api: Orders API [detail: Rust · Axum]
     db: Postgres [cylinder]
   }
   user -> api: POST /orders           # -> solid, ..> dashed, => thick, -- no arrow, <-> both
   api -> db
   ```
   Shapes: box, rounded, pill, circle, diamond, hexagon, cylinder, parallelogram, document, person, cloud, sticky, text.
   Colours: gray, blue, green, yellow, orange, red, purple, pink, teal.

   Blocks, for what boxes and arrows draw badly (each is `KIND id: Title { … }`; `shape spec` has each one's lines):
   - `mindmap` (an indented outline), `timeline` (`2026-10-28: Open beta [important]`), `cycle`, `adr` (decision record: `context:`, `options:`, `decision:`, `consequences:`; status `[accepted]`, `[proposed]`…)
   - `todo` (`[ ] Item [owner: ana, due: 28 Oct]`), `kanban` (with `column` blocks), `gantt` (`Task [2w, after: design, done: 40%]`, `[milestone]`; stages indented under a sub-project, named by what they deliver, not *Phase 2*), `risks` (`Risk [p: 4, i: 3, owner: sam]`)
   - `table`, `chart` (`[line]`, `[bar]`, `[donut]`… over a small table), `forecast` (revenue `now: 12k`, then `Launch milestone [adds: 3k, ramp: 3]`: it follows the gantt's dates)
   - `entity` (a database table: `email: text [unique]`, `team_id: uuid [fk: teams]`), `schema` (tables read from a file: create it with `shape schema FILE db/structure.sql`)
   - `sequence` (participants, then `a -> b: message`), `lanes` (swimlanes), `screen` (a wireframe for the device the product runs on, `screen login: Sign in [web] {` for a web app or a widget in one, `[mobile]` for a phone app; then one component per line: `title: Welcome back`, `input email: Email`, `button go: Sign in [blue]`)
   - `canvas id: Name { … }` holds a whole board inside this one; `type: product` or `type: project` gives the board perspectives (Delivery, Product, Business…), each opening on a summary Shape works out from the blocks.
2. Gather the facts first. When drawing a codebase, read the code (entry points, services, queues, databases, external APIs) and draw what is actually there. Use short labels, put tech details in `detail:`, and group by deployable unit or bounded context.
3. Write the file, e.g. `docs/diagrams/<name>.shape`. Leave out `x`/`y`, because the layout places nodes automatically.
4. Validate it, then render it:
   ```bash
   shape check docs/diagrams/checkout.shape
   shape png docs/diagrams/checkout.shape -o docs/diagrams/checkout.png   # or: shape svg …
   ```
   Look at the PNG. If it is cramped, try `direction: down`, fewer edges or groups.
5. Offer to open the editor: `shape docs/diagrams/checkout.shape` (run it in the background; it serves on localhost and opens the browser).

## Tips

- Prefer 5–20 nodes. Split a big system into several diagrams.
- Edge labels should name the interaction (`charge`, `order.created`), not repeat the node names.
- Use `comment on <id>: …` to flag open questions you could not answer from the code.
- `shape convert FILE --to mermaid` gives a Mermaid version for GitHub Markdown.

Tag what each audience needs (`[tags: product]`, `[tags: delivery]`); sections and blocks take tags too. Tag what is technical `tech` (architecture, hosting, APIs, data stores, implementation decisions): the Product perspective leaves it out, so the product scope reads without implementation details. A board that is scoping work needs a delivery plan: if the person has not said who does it and by when, ask them (the team, the start, fixed dates, milestones, platforms), or leave the questions in a to-do list for ✦ Plan the delivery.
