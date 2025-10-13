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
        msrv = packageToml.rust-version;
      in
      {
        packages.default = pkgs.callPackage ./default.nix {
          version = packageToml.version;
          rust = pkgs.rust-bin.stable.latest.minimal;
        };

        devShells = pkgs.callPackages ./shell.nix {
          package = self.packages.${system}.default;

          rust = {
            default = pkgs.rust-bin.stable.latest.minimal;
            msrv = pkgs.rust-bin.stable.${msrv}.minimal;
          };
        };

        formatter = pkgs.nixfmt-tree;
      }
    );
}
