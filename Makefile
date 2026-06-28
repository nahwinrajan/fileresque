.PHONY: dev build test smoke lint coverage clean

dev:
	bun tauri dev

build:
	bun tauri build

test:
	cargo test --workspace
	bun vitest run

# Runtime smoke check — boots the frontend headless and asserts it mounts with no
# console errors. Catches blank-page / crash-on-mount that unit tests miss.
# Required before any QA sign-off (see docs/agent-planning/decisions.md DECISION-014).
smoke:
	bash scripts/smoke.sh

lint:
	cargo clippy --workspace --all-targets -- -D warnings
	cargo fmt --all -- --check
	bun biome ci .

coverage:
	cargo llvm-cov --workspace --lcov --output-path lcov.info
	bun vitest run --coverage

clean:
	cargo clean
	rm -rf dist .svelte-kit node_modules
