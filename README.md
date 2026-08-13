# asciinema

[![Build Status](https://github.com/asciinema/asciinema/actions/workflows/ci.yml/badge.svg)](https://github.com/asciinema/asciinema/actions/workflows/asciinema.yml)
[![license](http://img.shields.io/badge/license-GNU-blue.svg)](https://raw.githubusercontent.com/asciinema/asciinema/master/LICENSE)

__asciinema__ (aka asciinema CLI or asciinema recorder) is a command-line tool
for recording and live streaming terminal sessions.

Unlike typical _screen_ recording software, which records visual output of a
screen into a heavyweight video files (`.mp4`, `.mov`), asciinema CLI runs
_inside a terminal_, capturing terminal session output into a lightweight
recording files in the
[asciicast](https://docs.asciinema.org/manual/asciicast/v3/) format (`.cast`),
or streaming it live to viewers in real-time.

The recordings can be replayed in a terminal, embedded on a web page with the
[asciinema player](https://docs.asciinema.org/manual/player/), or published to
an [asciinema server](https://docs.asciinema.org/manual/server/), such as
[asciinema.org](https://asciinema.org), for further sharing. Live streams allow
viewers to watch terminal sessions as they happen.

asciinema runs on GNU/Linux, macOS and FreeBSD.

<a href="https://asciinema.org/a/756853"><img src="https://asciinema.org/a/756853.svg" alt="asciinema CLI demo" width="100%" /></a>

Notable features:

- recording of terminal sessions to a file, with optional [keyboard input
  capture](https://docs.asciinema.org/manual/cli/quick-start/) and configurable
  environment variable capture,
- replaying of recordings inside a terminal, with adjustable speed, looping,
  idle time limiting, step-by-step navigation,
  pause-on-[markers](https://docs.asciinema.org/manual/cli/markers/), and
  optional terminal auto-resize,
- local and remote [live
  streaming](https://docs.asciinema.org/manual/cli/quick-start/#stream-a-terminal-session)
  of terminal sessions to multiple viewers in real-time, including a built-in
  HTTP server with an embedded web player for LAN/localhost viewing,
- combined sessions: record to a file while streaming locally and remotely at
  the same time,
- [lightweight asciicast recording
  format](https://docs.asciinema.org/manual/asciicast/v3/), with native reading
  and writing of zstd-compressed (`.zst`) recordings (8% of the original size
  on average),
- conversion from asciicast v1/v2/v3 to asciicast v2/v3, raw terminal output,
  or plain text,
- concatenation of multiple recordings into one, with timing adjusted
  automatically,
- mid-session controls: pause/resume capture and add markers on the fly via
  [customizable key bindings](https://docs.asciinema.org/manual/cli/configuration/),
- session metadata capture, including terminal size, terminal theme, command,
  and title,
- configuration file support for defaults such as recording command, capture
  options, playback speed, idle time limit, notifications, and key bindings,
- headless mode, configurable terminal window size, and exit-status propagation
  for scripted and CI-friendly recording and streaming,
- support for stdin/stdout in conversion and playback from local files, stdin,
  or HTTP(S) URLs,
- integration with [asciinema
  server](https://docs.asciinema.org/manual/server/), e.g.
  [asciinema.org](https://asciinema.org), for uploads, hosting, remote live
  streaming, self-hosted servers, visibility control, descriptions, and
  synchronized audio URLs.

To record a session run this command in your shell:

```sh
asciinema rec demo.cast
```

To stream a session via built-in HTTP server run:

```sh
asciinema stream -l
```

To stream a session via a relay (asciinema server) run:

```sh
asciinema stream -r
```

Check out the [Getting started
guide](https://docs.asciinema.org/getting-started/) for installation and usage
overview.

## Building

To download the source code, build the asciinema binary, and install it in
`$HOME/.cargo/bin` in one go run:

```sh
cargo install --locked --git https://github.com/asciinema/asciinema
```

This requires the Rust toolchain (1.82 or later) with Cargo, installed via your
system package manager or [rustup](https://rustup.rs/).

Once installed, ensure `$HOME/.cargo/bin` is in your shell's `$PATH`.

Alternatively, you can build from a local checkout. First download the source
code:

```sh
git clone https://github.com/asciinema/asciinema
cd asciinema
```

The recommended way to get the toolchain is the Nix dev shell, which provides
everything needed and just works:

```sh
nix develop
```

If you don't use Nix, you need Rust with Cargo, as above.

Then build the binary with:

```sh
cargo build --release
```

This produces the binary at `target/release/asciinema`. You can just copy the
binary to a directory in your `$PATH`.

To generate man pages and shell completion files, set `ASCIINEMA_GEN_DIR` to the
path where these artifacts should be stored. For example:

```sh
ASCIINEMA_GEN_DIR=/foo cargo build --release
```

The above command will build the binary and place the man pages in `/foo/man/`,
and the shell completion files in the `/foo/completion/` directory.

> [!NOTE]
> Windows is currently not supported. See [#467](https://github.com/orgs/asciinema/discussions/278).
> You can try [PowerSession](https://github.com/Watfaq/PowerSession-rs) instead.

## Development

All development happens on `develop` branch. This branch contains the current
generation (3.x) of the asciinema CLI, written in Rust.

The previous generation (2.x), written in Python, can be found in the `python`
branch.

For toolchain setup and build instructions, see [Building](#building). Run the
test suite with `cargo test`.

If you'd like to propose or submit any changes, please read the
[contribution guidelines](CONTRIBUTING.md) first.

## Donations

Sustainability of asciinema development relies on donations and sponsorships.

If you like the project then consider becoming a
[supporter](https://docs.asciinema.org/donations/#individuals) or a [corporate
sponsor](https://docs.asciinema.org/donations/#corporate-sponsorship).

asciinema is sponsored by:

- [Brightbox](https://www.brightbox.com/)

## Consulting

If you're interested in integration or customization of asciinema to suit your
needs, check [asciinema consulting
services](https://docs.asciinema.org/consulting/).

## License

© 2011 Marcin Kulik.

All code is licensed under the GPL, v3 or later. See [LICENSE](./LICENSE) file
for details.
