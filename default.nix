{
  lib,
  stdenv,
  rust-bin, # From overlay
  makeRustPlatform,
  packageToml,
  rust,
  libiconv,
  darwin,
  python3,
}: let
  testDeps = [
    python3
  ];

  buildDeps = rust:
    [
      rust
    ]
    ++ (lib.optionals stdenv.isDarwin [
      libiconv
      darwin.apple_sdk.frameworks.Foundation
    ])
    ++ testDeps;

  mkPackage = rust:
    (makeRustPlatform {
      cargo = rust;
      rustc = rust;
    })
    .buildRustPackage {
      pname = packageToml.name;
      inherit (packageToml) version;
      src = builtins.path {
        path = ./.;
        inherit (packageToml) name;
      };
      cargoLock.lockFile = ./Cargo.lock;
      buildInputs = buildDeps rust;
      dontUseCargoParallelTests = true;
    };
in (mkPackage rust-bin.stable.latest.minimal)
