{
  package,
  shellcheck,
  mkShell,
  rust,
}:
let
  mkDevShell =
    rust:
    mkShell {
      inputsFrom = [
        (package.override {
          rust = rust.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "clippy"
            ];
          };
        })
      ];

      packages = [ shellcheck ];

      env.RUST_BACKTRACE = 1;
    };
in
{
  default = mkDevShell rust.default;
  msrv = mkDevShell rust.msrv;
}
