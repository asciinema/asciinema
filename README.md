# asciinema

[![Build Status](https://github.com/asciinema/asciinema/actions/workflows/asciinema.yml/badge.svg)](https://github.com/asciinema/asciinema/actions/workflows/asciinema.yml)
[![PyPI](https://img.shields.io/pypi/v/asciinema.svg)](https://pypi.org/project/asciinema/)
[![license](http://img.shields.io/badge/license-GNU-blue.svg)](https://raw.githubusercontent.com/asciinema/asciinema/master/LICENSE)

__asciinema__ (aka asciinema CLI or asciinema recorder) is a command-line tool
for recording terminal sessions.

Unlike typical _screen_ recording software, which records visual output of a
screen into a heavyweight video files (`.mp4`, `.mov`), asciinema recorder runs
_inside a terminal_, capturing terminal session output into a lightweight
recording files in the
[asciicast](https://docs.asciinema.org/manual/asciicast/v2/) format (`.cast`).

The recordings can be replayed in a terminal, embedded on a web page with the
[asciinema player](https://docs.asciinema.org/manual/player/), or published to
an [asciinema server](https://docs.asciinema.org/manual/server/), such as
[asciinema.org](https://asciinema.org), for further sharing.

[![asciinema CLI
demo](https://asciinema.org/a/85R4jTtjKVRIYXTcKCNq0vzYH?cols=80)](https://asciinema.org/a/85R4jTtjKVRIYXTcKCNq0vzYH?autoplay=1)

Notable features:

* [recording](https://docs.asciinema.org/manual/cli/usage/#asciinema-rec-filename)
  and
  [replaying](https://docs.asciinema.org/manual/cli/usage/#asciinema-play-filename)
  of sessions inside a terminal,
* [light-weight recording
  format](https://docs.asciinema.org/manual/asciicast/v2/), which is highly
  compressible (down to 15% of the original size e.g. with `zstd` or `gzip`),
* integration with [asciinema
  server](https://docs.asciinema.org/manual/server/), e.g.
  [asciinema.org](https://asciinema.org), for easy recording hosting.

Recording is as easy as running this command in your shell:

```sh
asciinema rec demo.cast
```

Check out the [Getting started
guide](https://docs.asciinema.org/getting-started/) for installation and usage
overview.

## Donations

Sustainability of asciinema development relies on donations and sponsorships.

Please help the software project you use and love. Become a
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
