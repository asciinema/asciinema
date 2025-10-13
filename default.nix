{
  lib,
  stdenv,
  rust,
  makeRustPlatform,
  version,
  libiconv,
  darwin,
  python3,
}:
(makeRustPlatform {
  cargo = rust;
  rustc = rust;
}).buildRustPackage
  {
    pname = "asciinema";
    inherit version;

    src = builtins.path {
      path = ./.;
      name = "asciinema";
    };

    dontUseCargoParallelTests = true;
    cargoLock.lockFile = ./Cargo.lock;
    nativeBuildInputs = [ rust ];

    buildInputs = lib.optional stdenv.isDarwin [
      libiconv
      darwin.apple_sdk.frameworks.Foundation
    ];

    nativeCheckInputs = [ python3 ];
  }
