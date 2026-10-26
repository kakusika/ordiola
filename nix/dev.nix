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
    fenix.targets.aarch64-linux-android.stable.rust-std
    fenix.targets.armv7-linux-androideabi.stable.rust-std
    fenix.targets.i686-linux-android.stable.rust-std
    fenix.targets.x86_64-linux-android.stable.rust-std
  ];

  androidSdk = pkgs.androidenv.composeAndroidPackages {
    platformVersions = [
      "34"
      "36"
    ];
    buildToolsVersions = [
      "34.0.0"
      "36.0.0"
    ];
    includeNDK = true;
  };
  androidHome = "${androidSdk.androidsdk}/libexec/android-sdk";

  runtimeLibraries = with pkgs; [
    gtk3
    gtk4
    webkitgtk_4_1
    webkitgtk_6_0
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
  buildInputs =
    with pkgs;
    [
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

      #= Java / Android Tooling
      jdk
      androidSdk.androidsdk

      #= Tauri v2 Build Tools
      cargo-tauri
      pkg-config
      gobject-introspection
    ]
    ++ runtimeLibraries;

  ANDROID_HOME = androidHome;
  ANDROID_NDK_ROOT = "${androidHome}/ndk-bundle";
  NDK_HOME = "${androidHome}/ndk-bundle";
  JAVA_HOME = pkgs.jdk.home;

  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.dbus.dev}/lib/pkgconfig:${pkgs.webkitgtk_6_0.dev}/lib/pkgconfig:${pkgs.gtk4.dev}/lib/pkgconfig";
  LD_LIBRARY_PATH = runtimeLibPath;

  # Embed RPATH into compiled binaries so they run without LD_LIBRARY_PATH
  "CARGO_TARGET_${rustTargetUpper}_RUSTFLAGS" = "-C link-arg=-Wl,-rpath,${runtimeLibPath}";

  RUST_BACKTRACE = 1;

  shellHook = ''
    export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH:$XDG_DATA_DIRS"
    echo "🪟 telaion - Tomet Workspace Document Editor"
  '';
}
