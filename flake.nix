{
  description = "Terminal session recorder";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        packageToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;

        defaultPackage = pkgs.callPackage ./default.nix {
          inherit packageToml;
          rust = pkgs.rust-bin.stable.latest.minimal;
        };
      in
      {
        formatter = pkgs.nixfmt-tree;

        packages.default = defaultPackage;

        devShells = pkgs.callPackages ./shell.nix {
          inherit packageToml;
          defaultPackage = defaultPackage;
        };
      }
    );
}
