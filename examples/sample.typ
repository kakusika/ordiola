#set page(paper: "a4", margin: (x: 2cm, y: 2.5cm))
#set text(font: "Linux Libertine", size: 11pt)

= Welcome to Typst in Ordiola

This is a real-time live preview of *Typst 0.15* within *Ordiola*, powered by Tauri v3, Svelte 5, and the `@ordiola/ext-typst` extension.

== Mathematical Typesetting

Typst makes beautiful math formulas effortless:

$ integral_0^infinity e^(-x^2) dif x = sqrt(pi) / 2 $

$ A = mat(
  1, 2, 3;
  4, 5, 6;
  7, 8, 9;
) $

== Code Blocks and Highlighting

```rust
fn main() {
    println!("Hello from Ordiola with Typst!");
}
```

== Features

- *Sub-millisecond Compilation*: Native Rust `typst` engine directly in the Tauri backend.
- *Vector SVG Rendering*: Infinite zoom and crisp typography.
- *Real-time Reactivity*: Fast debounced updates on keystrokes.
- *PDF Export*: Direct generation via `typst-pdf`.
