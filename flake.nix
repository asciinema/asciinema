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
        config,
        self',
        inputs',
        pkgs,
        system,
        ...
      }: let
        packageToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
        msrv = packageToml.rust-version;

        mkDevShell = rust:
          pkgs.mkShell {
            inputsFrom = [
              (config.packages.default.override {
                rust = rust.override {
                  extensions = ["rust-src"];
                };
              })
            ];

            env.RUST_BACKTRACE = 1;
          };
      in {
        _module.args = {
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [(import rust-overlay)];
          };
        };

        formatter = pkgs.alejandra;

        devShells = {
          default = mkDevShell pkgs.rust-bin.stable.latest.default;
          msrv = mkDevShell pkgs.rust-bin.stable.${msrv}.default;
        };

        packages.default = pkgs.callPackage ./default.nix {inherit packageToml;};
      };
    };
}
