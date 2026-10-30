# ordiola development tasks.
# Run `just` to see everything available.

default:
    @just --list

#[ Build ]

# Debug build of Rust workspace
build:
    cargo build

# Optimized build of Rust workspace
release:
    cargo build --release

dist := "dist"
package := "com.ordiola.ordiola"
activity := package + "/.MainActivity"

# Clean all aggregated build outputs in dist/
clean-dist:
    rm -rf {{dist}}
    @echo "Cleaned {{dist}}/"

#[ Desktop ]

# Run desktop application in development mode
desktop-dev:
    pnpm tauri dev

# Build desktop standalone release binary and copy into dist/
desktop-release:
    pnpm tauri build --no-bundle
    @mkdir -p {{dist}}
    cp --remove-destination target/release/ordiola {{dist}}/ordiola
    @echo "Built: {{dist}}/ordiola"

# Build and run desktop release binary (with optional vault path)
desktop-run *ARGS: desktop-release
    ./{{dist}}/ordiola {{ARGS}}

#[ Android ]

# Run Android application in development mode (device or emulator)
android-dev *ARGS:
    pnpm tauri android dev {{ARGS}}

# Build Android APK (debug) and copy into dist/
android-build *ARGS:
    pnpm tauri android build --debug --apk {{ARGS}}
    @mkdir -p {{dist}}
    find apps/tauri/gen/android/app/build/outputs/apk -name "*debug*.apk" -exec cp --remove-destination {} {{dist}}/ordiola-debug.apk \;
    @echo "Built: {{dist}}/ordiola-debug.apk"

android_cflags := "-fno-unwind-tables -fno-asynchronous-unwind-tables -ffunction-sections -fdata-sections"
android_rustflags := "-C force-unwind-tables=no -C link-arg=-Wl,--icf=all -C link-arg=-Wl,--gc-sections -C link-arg=-Wl,--strip-all"

# Build Android APK (release, aarch64 by default) and copy into dist/
android-release *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    args=("{{ARGS}}")
    target_opt=""
    if [[ ! " ${args[*]} " =~ " -t " && ! " ${args[*]} " =~ " --target " ]]; then
        target_opt="-t aarch64"
    fi
    RUSTFLAGS="{{android_rustflags}}" CFLAGS="{{android_cflags}}" pnpm tauri android build --apk ${target_opt} {{ARGS}}
    mkdir -p {{dist}}
    find apps/tauri/gen/android/app/build/outputs/apk -name "*release*.apk" -exec cp --remove-destination {} {{dist}}/ordiola.apk \;
    echo "Built: {{dist}}/ordiola.apk"

# Build Android fat APK containing all architectures
android-release-universal *ARGS:
    RUSTFLAGS="{{android_rustflags}}" CFLAGS="{{android_cflags}}" pnpm tauri android build --apk {{ARGS}}
    @mkdir -p {{dist}}
    find apps/tauri/gen/android/app/build/outputs/apk -name "*release*.apk" -exec cp --remove-destination {} {{dist}}/ordiola-universal.apk \;
    @echo "Built: {{dist}}/ordiola-universal.apk"

# Install the built debug APK on connected Android device/emulator
android-install:
    adb install -r {{dist}}/ordiola-debug.apk

# Install and launch on connected Android device/emulator
android-launch: android-install
    adb shell am start -n {{activity}}

#[ Frontend ]

# Run frontend development server
frontend-dev:
    pnpm --filter @ordiola/frontend dev

# Build frontend production bundle
frontend-build:
    pnpm --filter @ordiola/frontend build

# Install frontend dependencies
frontend-install:
    pnpm install

#[ Aliases & Shortcuts ]

# Run desktop application in development mode (alias for desktop-dev)
alias dev := desktop-dev

# Run built desktop binary directly without rebuilding
run *ARGS:
    @if [ -f {{dist}}/ordiola ]; then ./{{dist}}/ordiola {{ARGS}}; else ./target/release/ordiola {{ARGS}}; fi

# Backward-compatible aliases
alias tauri-build := desktop-release
alias tauri-run := desktop-run

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
