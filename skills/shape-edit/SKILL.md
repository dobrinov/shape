---
name: shape-edit
description: Make precise changes to an existing .shape (or diagram JSON / Mermaid) file — add or remove nodes and edges, regroup, restyle, rename ids — with the `shape apply` ops API, keeping the file valid. Use when the user asks to change, update, extend or clean up an existing diagram.
---

# shape-edit

## Steps

1. Read only what you need: `shape query FILE group:backend` (or `near:api`, `chart:plan`, `map:goals`, `label~ledger`, `kind:task`, `owner:ana`, `comments:open`; `--ids` for ids only) prints that part of the board as Shape language. For the whole board, `cat FILE` or `shape brief FILE`.
2. Adding a lot: write statements and merge them, e.g. `shape add FILE - <<'EOF'` … `EOF` with lines like `group backend {` / `  cache: Redis [cylinder]` / `}` / `api -> cache: reads` / `api [red]`. New elements are created; one written again by its id gets the attributes given; a group written again keeps its place. (The op form is `{"op":"merge","dsl":"…"}`.)
3. Small changes: edit the text directly. Blocks (`todo`, `kanban`, `gantt`, `timeline`, `table`, `chart`, `forecast`, `adr`, `screen`, `entity`…) list their items one per line, so ticking `[ ]` → `[x]`, moving a card to another `column` or changing a task's `done:` is a one-line edit. Tables in a `schema` block come from a file: change the file and run `shape schema FILE` instead. The format is one statement per line, so this is easy and diffs cleanly. Then run `shape check FILE`.
4. Structural changes (renames, moving things between groups, deleting nodes together with their edges): use ops. A rename updates every edge and comment that refers to the old id.
   ```bash
   shape apply FILE - <<'JSON'
   {"ops": [
     {"op": "rename", "id": "api", "to": "orders-api"},
     {"op": "update_node", "id": "db", "shape": "cylinder", "color": "blue"},
     {"op": "add_group", "id": "data", "label": "Data", "members": ["db", "cache"]},
     {"op": "remove", "id": "legacy"},
     {"op": "set", "key": "direction", "value": "down"}
   ]}
   JSON
   ```
   `shape apply` prints a summary of what changed. Add `--dry-run` to preview it first. One op can go inline: `shape apply FILE '{"op":"update_node","id":"db","color":"red"}'`. A mistyped id gets a suggestion ("did you mean `orders-api`?").
5. Look at one part: `shape png FILE --focus backend,api -o /tmp/part.png` crops to those elements.
6. Re-render any committed images, e.g. `shape png FILE -o FILE.png`.
7. To show the user the change visually, run `shape svg FILE --base HEAD -o /tmp/changes.svg`. It highlights additions in green, removals in red and changes in amber.

## Gotchas

- `# …` lines are notes for people. Edits keep them (only changed statements are rewritten); `shape fmt` drops them. Use `comment` for review notes.
- Edge ids are `from->to` (`from->to#2` for a second edge between the same pair). Get them from `shape brief`.
- `x`/`y` pin a node. Use `{"op": "unpin"}` to hand every node back to the automatic layout.
