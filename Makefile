.PHONY: dev build test lint coverage clean

dev:
	bun tauri dev

build:
	bun tauri build

test:
	cargo test --workspace
	bun vitest run

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
