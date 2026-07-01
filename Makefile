.PHONY: dev build bundle test smoke lint coverage clean

dev:
	bun tauri dev

build:
	bun tauri build

# Bundle a macOS installer (.dmg) from a release build. This is an unsigned /
# ad-hoc build for local install + testing — full code signing + notarisation
# (P5-T01) need Apple certs supplied via env (APPLE_SIGNING_IDENTITY,
# APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID). Runs `bun run build` (vite) first
# via the beforeBuildCommand hook, then produces the .dmg.
#   Output: src-tauri/target/release/bundle/dmg/FileResque_<version>_<arch>.dmg
bundle:
	bun tauri build --bundles dmg
	@echo "→ installer written to: src-tauri/target/release/bundle/dmg/"

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
