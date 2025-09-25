{
  lib,
  stdenv,
  rust,
  makeRustPlatform,
  packageToml,
  libiconv,
  darwin,
  python3,
}:
(makeRustPlatform {
  cargo = rust;
  rustc = rust;
}).buildRustPackage
  {
    pname = packageToml.name;
    inherit (packageToml) version;

    src = builtins.path {
      path = ./.;
      inherit (packageToml) name;
    };

    dontUseCargoParallelTests = true;

    cargoLock.lockFile = ./Cargo.lock;

    nativeBuildInputs = [ rust ];
    buildInputs = lib.optional stdenv.isDarwin [
      libiconv
    ];

    nativeCheckInputs = [ python3 ];
  }
