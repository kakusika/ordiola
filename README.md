<!-- Generated from tmtroot/readme.tmt. Edit that, then `tomet export .`. -->

# Telaion

Workspace document editor for Tomet with hybrid block and WYSIWYG editing, built on Tauri v2 and Svelte 5.

## Features

- **Tomet Native**: Read and edit `.tmt` documents natively with AST-level preservation.
- **Dual Mode Editing**:
  - **Block Hybrid Mode**: Edit markup inline within structured blocks (<callout>, @meta, etc.).
  - **Full Block Mode (WYSIWYG)**: Seamless rich-text rendering with ProseMirror / Tiptap.
- **Workspace Navigation**: Browse and organize files in vaults.
- **Powered by Tauri v2 & Svelte 5**: Ultra-fast and lightweight desktop application.

## Development

### Prerequisites

- Nix with Flakes enabled (recommended)
- Rust toolchain (MSRV 1.85+)
- Node.js & pnpm
- WebKitGTK 4.1 & GTK3 development libraries (Linux)

### Using Nix Flake

```bash
direnv allow
# or
nix develop
```

### Running Locally

```bash
just frontend-install
just dev
```

