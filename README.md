# FileResque

A native file recovery application built with Tauri 2 and Rust. Scan disks for deleted files, assess recovery probability, and restore them safely to another drive.

**Platforms:** macOS (primary) · Windows

---

## Prerequisites

### macOS

| Tool | Version | Install |
|------|---------|---------|
| Rust | ≥ 1.80 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | ≥ 20 LTS | https://nodejs.org or `brew install node` |
| npm | ≥ 10 | Bundled with Node.js |
| Xcode CLI Tools | Latest | `xcode-select --install` |
| Homebrew (optional) | Latest | https://brew.sh |

### Windows

| Tool | Version | Install |
|------|---------|---------|
| Rust | ≥ 1.80 | https://rustup.rs |
| Node.js | ≥ 20 LTS | https://nodejs.org |
| Visual Studio Build Tools | 2022 | Install "Desktop development with C++" workload |
| WebView2 Runtime | Latest | Usually pre-installed on Windows 11; https://developer.microsoft.com/microsoft-edge/webview2 |

---

## Quick Start

```bash
# Clone the repository
git clone https://github.com/your-org/fileresque.git
cd fileresque

# Install Rust toolchain components
rustup component add clippy rustfmt

# Install cargo tools
cargo install cargo-llvm-cov cargo-audit cargo-deny

# Install frontend dependencies
npm install

# Run in development mode
make dev
```

The app window will open. On first launch, macOS will prompt you to grant **Full Disk Access** — follow the on-screen instructions.

---

## Available Make Targets

```bash
make dev          # Start Tauri dev server (hot-reloads frontend)
make build        # Production build (outputs to src-tauri/target/release/)
make test         # Run all Rust unit + integration tests
make test-watch   # Run tests in watch mode
make lint         # clippy + rustfmt check + svelte-check
make fmt          # Auto-format Rust and TypeScript
make coverage     # Generate coverage report (HTML at target/llvm-cov/html/index.html)
make audit        # cargo audit + cargo deny (dependency security check)
make fuzz         # Run fuzz targets (requires cargo-fuzz; macOS only)
make clean        # Remove build artifacts
```

---

## Project Structure

```
fileresque/
├── crates/
│   ├── core/        # Shared types, traits, error definitions
│   ├── disk/        # Disk enumeration and filesystem parsers (APFS, HFS+, NTFS)
│   └── recovery/    # Recovery probability engine and file recovery
├── src-tauri/       # Tauri app shell and IPC command handlers
├── src/             # Svelte + TypeScript frontend
│   └── lib/
│       ├── components/
│       └── stores/
└── docs/            # Technical documentation
```

---

## Running Tests

```bash
# All unit and integration tests
make test

# Tests for a specific crate
cargo test -p fileresque-disk

# With output (for debugging)
cargo test -p fileresque-recovery -- --nocapture

# Coverage report
make coverage
# Open target/llvm-cov/html/index.html in your browser

# Frontend tests
npm run test

# Frontend tests with coverage
npm run test:coverage
```

**Coverage requirements:** Rust ≥ 80% · Frontend ≥ 70% · Enforced in CI.

---

## macOS: Full Disk Access

The application requires Full Disk Access to read raw disk structures. Without it, disk scans will return no results.

To grant access:
1. Open **System Settings** → **Privacy & Security** → **Full Disk Access**
2. Click **+** and add `FileResque.app`
3. Restart the application

The app will guide you through this on first launch.

---

## Windows: Administrator Privileges

On Windows, the application must be run as Administrator to access raw disk devices. The installer and application manifest request elevation automatically via UAC prompt.

---

## Development Notes

### Linting

```bash
# Run clippy (must have zero warnings — enforced in CI)
cargo clippy --all-targets -- -D warnings

# Rustfmt check
cargo fmt --check

# TypeScript and Svelte
npm run check
```

### Adding a Dependency

Before adding any Rust crate, check whether the standard library or an already-approved crate covers the need. If you add a crate, add a `# DEPENDENCY JUSTIFICATION:` comment in `Cargo.toml` explaining why it is necessary.

### Code Style

- **Cognitive complexity:** Maximum 15 per function. Run `cargo clippy` — it will flag violations.
- **Error handling:** Use `?` and `thiserror`-derived error types. No `unwrap()` or `expect()` outside tests without a `// JUSTIFIED:` comment.
- **Unsafe:** Any `unsafe` block must include a `// SAFETY:` comment explaining the invariant that makes it sound.

### Fuzz Testing

```bash
# Requires cargo-fuzz and a nightly toolchain
rustup toolchain install nightly
cargo +nightly fuzz run apfs_parser
cargo +nightly fuzz run ntfs_parser
```

---

## Architecture Decision Records

See `docs/agent-planning/decisions.md` for a log of all architectural and product decisions with rationale.

---

## Contributing

1. Pick a task from `docs/features/feature-breakdown-phases.md`
2. Read the corresponding planning doc in `docs/agent-planning/`
3. Follow the code standards in `CONTEXT.md`
4. All PRs must pass CI (lint, test, coverage gates)
5. Tag a QA reviewer on your PR

---

## Licence

MIT © FileResque Contributors