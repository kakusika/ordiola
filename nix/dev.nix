{
  lib,
  pkgs,
  mkShell,
  fenix,
  tools,
  ...
}:
let
  rustToolchain = fenix.combine [
    (fenix.stable.withComponents [
      "cargo"
      "clippy"
      "rustc"
      "rust-src"
      "rustfmt"
      "rust-analyzer"
    ])
  ];

  runtimeLibraries = with pkgs; [
    gtk3
    webkitgtk_4_1
    glib
    glib-networking
    gdk-pixbuf
    cairo
    pango
    atk
    libsoup_3
    openssl
    dbus
    wayland
    libxkbcommon
  ];

  rustTargetUpper = lib.toUpper (
    builtins.replaceStrings [ "-" ] [ "_" ] pkgs.stdenv.hostPlatform.rust.rustcTarget
  );
  runtimeLibPath = lib.makeLibraryPath runtimeLibraries;
in
mkShell rec {
  buildInputs = with pkgs; [
    #= Ecosystem Tools (sisho, tomet, twrit, catasto)
    tools
    just

    #= Rust
    rustToolchain
    cargo-edit
    cargo-nextest

    #= Frontend (Node & pnpm)
    nodejs
    pnpm

    #= Tauri v2 Build Tools
    cargo-tauri
    pkg-config
    gobject-introspection
  ] ++ runtimeLibraries;

  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.dbus.dev}/lib/pkgconfig";
  LD_LIBRARY_PATH = runtimeLibPath;

  # Embed RPATH into compiled binaries so they run without LD_LIBRARY_PATH
  "CARGO_TARGET_${rustTargetUpper}_RUSTFLAGS" = "-C link-arg=-Wl,-rpath,${runtimeLibPath}";

  RUST_BACKTRACE = 1;

  shellHook = ''
    export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH:$XDG_DATA_DIRS"
    echo "🪟 telaion - Tomet Workspace Document Editor"
  '';
}
