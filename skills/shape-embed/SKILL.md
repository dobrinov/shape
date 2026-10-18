---
name: shape-embed
description: Use Shape as a library — render diagrams in a web page or docs site, embed the editor in an app, or call the Rust crate from a backend. Use when the user wants diagrams inside their own product, docs or Markdown.
---

# shape-embed

## In a web page (no build step)

```bash
shape embed -o public/shape.embed.js     # one file: JS + WASM (~1 MB, gzip ~300 KB)
```

```html
<script src="/shape.embed.js"></script>

<!-- read-only viewer with pan/zoom, comments, export -->
<div data-shape data-mode="view" style="height:420px"><script type="text/plain">
api -> db: SQL
</script></div>

<script>
  // full editor
  Shape.mount("#editor", { source, mode: "edit", author: "ana", onChange: (src) => save(src) });
  // or render to an SVG string
  Shape.render("a -> b").then((svg) => (el.innerHTML = svg));
  // Markdown sites: turn ```shape fences into pictures
  Shape.renderAll("pre > code.language-shape");
</script>
```

The editor instance has `getSource(fmt)`, `setSource`, `apply(ops)`, `setBase(src)` (highlights changes), `prompt(opts)`, `exportSVG()`, `exportPNG()`, `on("change"|"select", fn)` and `destroy()`.

## Standalone HTML

`shape html FILE -o page.html` writes one self-contained file with the viewer and the diagram. You can attach it to a PR or send it in chat.

## Rust

```toml
shape-core = { git = "https://github.com/dobrinov/shape" }
```

```rust
let doc = shape_core::parse(&source)?;                     // DSL, JSON or Mermaid
let svg = shape_core::render_svg(&doc, &Default::default());
let prompt = shape_core::prompt::prompt(&doc, &Default::default());
shape_core::ops::apply(&mut doc, &shape_core::ops::parse_ops(llm_reply)?)?;
```

A server should store the DSL text: it is small, diffable and readable by both people and LLMs.
