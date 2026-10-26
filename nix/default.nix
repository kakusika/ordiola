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
    { system, ... }:
    let
      pkgs = import inputs.nixpkgs {
        inherit system;
        config = {
          allowUnfree = true;
          android_sdk.accept_license = true;
        };
      };
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
