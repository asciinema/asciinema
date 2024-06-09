{
  description = "Terminal session recorder";

  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs/nixos-unstable;
    rust-overlay.url = github:oxalica/rust-overlay;
    flake-parts.url = github:hercules-ci/flake-parts;
  };

  outputs = inputs @ {
    flake-parts,
    rust-overlay,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin"];
      perSystem = {
        self',
        pkgs,
        system,
        ...
      }: let
        packageToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
      in {
				formatter = pkgs.alejandra;

        _module.args = {
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [(import rust-overlay)];
          };
        };

        devShells = pkgs.callPackages ./shell.nix {inherit packageToml self';};

        packages.default = pkgs.callPackage ./default.nix {inherit packageToml;};
      };
    };
}
