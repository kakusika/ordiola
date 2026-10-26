{
  flake-parts,
  ...
}@inputs:
flake-parts.lib.mkFlake { inherit inputs; } {
  systems = [
    "x86_64-linux"
    "aarch64-linux"
    "aarch64-darwin"
  ];
  imports = [
    inputs.treefmt-nix.flakeModule
  ];

  perSystem =
    { pkgs, ... }:
    let
      system = pkgs.stdenv.hostPlatform.system;
      sisho = inputs.sisho.packages.${system};
    in
    {
      devShells.default = pkgs.callPackage ./dev.nix {
        inherit inputs;
        fenix = inputs.fenix.packages.${system};
        tools = inputs.sisho.legacyPackages.${system}.combine [
          sisho.sisho
          sisho.tomet
          sisho.tomet-lsp
          sisho.twrit
          sisho.catasto
        ];
      };

      treefmt = import ./formatter.nix {
        inherit (sisho) tomet;
      };
    };
}
