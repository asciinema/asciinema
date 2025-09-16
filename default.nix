{
  lib,
  stdenv,
  rust,
  makeRustPlatform,
  version,
  libiconv,
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
    ];

    nativeCheckInputs = [ python3 ];
  }
