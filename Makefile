# Shape build helpers
CARGO_BIN := $(HOME)/.cargo/bin
# Prefer the rustup toolchain (has the wasm32 target) over e.g. Homebrew's rustc.
RUSTUP_BIN := $(shell rustup which rustc 2>/dev/null | xargs dirname 2>/dev/null)
WASM_ENV := PATH="$(RUSTUP_BIN):$(CARGO_BIN):$$PATH" DYLD_FALLBACK_LIBRARY_PATH="$(RUSTUP_BIN)/../lib" CARGO_TARGET_DIR="$(CURDIR)/target/wasm"

# builds and tests here do not rewrite the Claude skills installed on this machine
export SHAPE_NO_SKILL_REFRESH := 1

.PHONY: all wasm js build release install test e2e site dev embed clean

all: build

wasm:
	$(WASM_ENV) wasm-pack build crates/wasm --release --target no-modules --no-typescript --no-pack --out-dir ../../web/pkg

# npm package in js/: ES-module WASM (Node >= 18 and browsers) into js/pkg
js:
	$(WASM_ENV) CARGO_TARGET_DIR="$(CURDIR)/target/wasm-web" wasm-pack build crates/wasm --release --target web --no-typescript --no-pack --out-dir ../../js/pkg

build: wasm
	SHAPE_SKIP_WASM=1 cargo build

release: wasm
	SHAPE_SKIP_WASM=1 cargo build --release

install: wasm
	SHAPE_SKIP_WASM=1 cargo install --path crates/cli --force

test:
	cargo test --workspace
	node --test web/test/*.test.mjs
	@# the js/ package tests need js/pkg (`make js`); skipped when it is not built
	@if [ -f js/pkg/shape_wasm_bg.wasm ]; then node --test js/test/*.test.mjs; else echo "skipping js/ tests: run 'make js' first"; fi

# Browser tests of the editor (Playwright + the installed Chrome; serves web/ from disk)
e2e: build
	cd scripts/e2e && (test -d node_modules || npm install --silent) && node mindmap.mjs && node risks.mjs && node charts.mjs && node tables.mjs && node themes.mjs && node ask.mjs && node board.mjs && node entity.mjs && node links.mjs && node kanban.mjs && node adr.mjs && node sequence.mjs && node images.mjs && node canvases.mjs && node merge.mjs && node timeline.mjs && node perspectives.mjs && node cycle.mjs && node history.mjs && node prompts.mjs && node ai-setup.mjs && node minimap.mjs && node presentation.mjs && node wireframes.mjs && node todo.mjs && node handles.mjs && node schema.mjs && node forecast.mjs && node images-ai.mjs && node busy.mjs && node programme.mjs

# Browser library: one JS file with the WASM inlined
embed: build
	./target/debug/shape embed -o shape.embed.js

# Static GitHub Pages site in ./docs (landing page, playground, examples)
site: release
	./scripts/build-site.sh

# Serve the frontend from disk while hacking on web/ (no rebuild needed)
dev: build
	SHAPE_WEB_DIR=web ./target/debug/shape examples/example.shape

clean:
	cargo clean
	rm -rf web/pkg js/pkg
