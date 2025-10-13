{
  defaultPackage,
  shellcheck,
  packageToml,
  rust-bin,
  mkShell,
}:
let
  msrv = packageToml.rust-version;

  mkDevShell =
    rust:
    mkShell {
      inputsFrom = [
        (defaultPackage.override {
          rust = rust.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
          };
        })
      ];

      packages = [ shellcheck ];

      env.RUST_BACKTRACE = 1;
    };
in
{
  default = mkDevShell rust-bin.stable.latest.default;
  msrv = mkDevShell rust-bin.stable.${msrv}.default;
}
