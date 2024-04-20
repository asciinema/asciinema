{
  description = "Terminal session recorder";

  inputs = {
    nixpkgs.url = github:nixos/nixpkgs/nixpkgs-unstable;
    rust-overlay.url = github:oxalica/rust-overlay;
    flake-utils.url = github:numtide/flake-utils;
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        msrv = cargoToml.package.rust-version;

        buildDeps = rust: with pkgs; [
          rust
        ] ++ (lib.optionals stdenv.isDarwin [
          libiconv
          darwin.apple_sdk.frameworks.Foundation
        ]) ++ testDeps;

        testDeps = with pkgs; [
          python3
        ];

        mkDevShell = rust: pkgs.mkShell {
          nativeBuildInputs = buildDeps (rust.override {
            extensions = [ "rust-src" ];
          });

          RUST_BACKTRACE = 1;
        };

        mkPackage = rust: (pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        }).buildRustPackage {
          inherit (cargoToml.package) name version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = buildDeps rust;
          dontUseCargoParallelTests = true;
        };
      in
      {
        devShells.default = mkDevShell pkgs.rust-bin.stable.latest.default;
        devShells.msrv = mkDevShell pkgs.rust-bin.stable.${msrv}.default;
        packages.default = mkPackage pkgs.rust-bin.stable.latest.minimal;
      }
    );
}
