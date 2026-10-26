{
  lib,
  stdenv,
  rustPlatform,
  fetchPnpmDeps,
  cargo-tauri,
  nodejs,
  pnpm,
  pnpmConfigHook,
  pkg-config,
  wrapGAppsHook4,
  openssl,
  gtk3,
  webkitgtk_4_1,
  glib,
  glib-networking,
  libsoup_3,
}:
let
  src = ../..;
in
rustPlatform.buildRustPackage rec {
  inherit src;

  pname = "telaion";
  version = "0.1.0";

  cargoLock.lockFile = "${src}/Cargo.lock";
  cargoTestFlags = cargoBuildFlags;
  cargoBuildFlags = [
    "-p"
    "telaion-tauri"
  ];

  pnpmDeps = fetchPnpmDeps {
    pname = "${pname}-pnpm-deps-${version}";
    inherit src version;
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    fetcherVersion = 4;
  };

  nativeBuildInputs = [
    cargo-tauri.hook
    nodejs
    pnpm
    pnpmConfigHook
    pkg-config
  ]
  ++ lib.optionals stdenv.hostPlatform.isLinux [ wrapGAppsHook4 ];

  buildInputs = [
    webkitgtk_4_1
    openssl
    gtk3
    glib
    glib-networking
    libsoup_3
  ];

  meta = with lib; {
    description = "Telaion - Workspace document editor for Tomet";
    platforms = platforms.all;
    mainProgram = "telaion";
  };
}
