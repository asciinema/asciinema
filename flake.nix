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
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        msrv = cargoToml.package.rust-version;

        buildDeps = rust:
          with pkgs;
            [
              rust
            ]
            ++ (lib.optionals stdenv.isDarwin [
              libiconv
              darwin.apple_sdk.frameworks.Foundation
            ])
            ++ testDeps;

        testDeps = with pkgs; [
          python3
        ];

        mkDevShell = rust:
          pkgs.mkShell {
            nativeBuildInputs = buildDeps (rust.override {
              extensions = ["rust-src"];
            });

            RUST_BACKTRACE = 1;
          };

        mkPackage = rust:
          (pkgs.makeRustPlatform {
            cargo = rust;
            rustc = rust;
          })
          .buildRustPackage {
            inherit (cargoToml.package) name version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = buildDeps rust;
            dontUseCargoParallelTests = true;
          };
      in {
      	_module.args = {
      		pkgs = import inputs.nixpkgs {
      			inherit system;
      			overlays = [ (import rust-overlay) ];
      		};
      	};

        formatter = pkgs.alejandra;

        devShells = {
          default = mkDevShell pkgs.rust-bin.stable.latest.default;
          msrv = mkDevShell pkgs.rust-bin.stable.${msrv}.default;
        };
        packages.default = mkPackage pkgs.rust-bin.stable.latest.minimal;
      };
    };
}
