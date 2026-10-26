# telaion development tasks.
# Run `just` to see everything available.

default:
    @just --list

#[ Build ]

# Debug build of Rust workspace
build:
    cargo build

# Optimized build
release:
    cargo build --release

# Build standalone Tauri release binary
tauri-build:
    pnpm tauri build --no-bundle

# Build and run standalone Tauri release binary (with optional vault path)
tauri-run *ARGS: tauri-build
    ./target/release/telaion {{ARGS}}

# Run built release binary directly without rebuilding
run *ARGS:
    ./target/release/telaion {{ARGS}}

# Run frontend development server
frontend-dev:
    pnpm --filter @telaion/frontend dev

# Build frontend production bundle
frontend-build:
    pnpm --filter @telaion/frontend build

# Install frontend dependencies
frontend-install:
    pnpm install

#[ Run ]

# Run Tauri application in development mode
dev:
    pnpm tauri dev

#[ Test & Lint ]

# Run Rust test suite
test:
    cargo test --workspace

# Type-check without producing a binary
check:
    cargo check --all-targets

# Lint with clippy
clippy:
    cargo clippy --all-targets -- -D warnings

# Format every tracked file
fmt:
    nix fmt

# Everything a CI job would run
ci: check test
    nix fmt -- --ci
    cargo clippy --all-targets -- -D warnings
    git diff --exit-code Cargo.lock

#[ Documentation ]

# Generate and update all derived documentation across the workspace.
docs:
    sisho fix .

# Check that all documents parse, format cleanly, and match export targets.
docs-check:
    sisho check .
