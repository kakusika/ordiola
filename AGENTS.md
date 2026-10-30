<!-- Generated from tmtroot/agents.tmt. Edit that, then `tomet export .`. -->

# Ordiola

## Core Philosophy & Principles

### Precedence of Instructions

The rules in this document serve as the project baseline and defaults. Explicit instructions given by the user in a conversation always take precedence over these defaults.

### User-First Design over Developer Convenience

Always prioritize the end-user experience, syntax ergonomics, and API design.
If there is tension between developer or implementation convenience and user simplicity, user simplicity must always win. Never compromise user experience just because an implementation is easier.

### Resist Path-of-Least-Resistance & Legacy Bias

The existing implementation is never a justification or reason for new design choices.

- Do not justify decisions with "it is already implemented this way, so let's stick with it."
- Prioritize architectural beauty, elegance, and long-term correctness over the path of least resistance.
- Understand the existing context thoroughly, but never take shortcuts or settle for lazy patches.

### Transparent Execution

Always state what you intend to do before doing it. Never execute non-trivial actions or commands silently.

## Single Source of Truth & Documentation

### Never edit `.md` directly

All Markdown files (`README.md`, `AGENTS.md`, `CONTRIBUTING.md`) are generated build artifacts, not source files.

- Never edit a `.md` file directly.
- Always edit the corresponding `.tmt` file under `tmtroot/`.
- Run `just docs` (or `tomet export .`) to regenerate the Markdown artifacts, and verify with `just docs-check`.
- Direct edits to generated `.md` files will be overwritten and will fail CI / lint checks.

## Project overview

`ordiola` is a workspace document editor for Tomet (`.tmt`) documents, supporting both block hybrid editing and full block WYSIWYG editing, built on Tauri v2, Svelte 5, and Tiptap/ProseMirror.

Read `CONTRIBUTING.md` for the contribution workflow, commit conventions, and verification steps.

## Environment & Tooling

All commands, builds, and tests must run within the Nix development environment (via `direnv` or `nix develop`). Assume this environment is active and rely on the tools provided by the flake, rather than assuming or relying on global host toolchains.

