# asciinema

[![Build Status](https://travis-ci.org/asciinema/asciinema.svg?branch=master)](https://travis-ci.org/asciinema/asciinema)
[![license](http://img.shields.io/badge/license-GNU-blue.svg)](https://raw.githubusercontent.com/asciinema/asciinema/master/LICENSE)

Terminal session recorder and the best companion of
[asciinema.org](https://asciinema.org).

[![demo](https://asciinema.org/a/42383.png)](https://asciinema.org/a/42383?autoplay=1)

## Installation

### Native packages

asciinema is included in repositories of most popular package managers on Mac OS
X, Linux and FreeBSD. Look for package named `asciinema`. See the
[list of available packages](https://asciinema.org/docs/installation).

### Python package

asciinema is available on [PyPI](https://pypi.python.org/pypi/asciinema) and can
be installed with pip (Python 3 required):

    sudo pip3 install asciinema

### Running latest version from master

If none of the above works for you (or you want to help with development) just
clone the repo and run asciinema straight from the checkout:

    git clone https://github.com/asciinema/asciinema.git
    cd asciinema
    python3 -m asciinema --version

## Usage

asciinema is composed of multiple commands, similar to `git`, `apt-get` or
`brew`.

When you run `asciinema` with no arguments help message is displayed, listing
all available commands with their options.

### `rec [filename]`

__Record terminal session.__

This is the single most important command in asciinema, since it is how you
utilize this tool's main job.

By running `asciinema rec [filename]` you start a new recording session. The
command (process) that is recorded can be specified with `-c` option (see
below), and defaults to `$SHELL` which is what you want in most cases.

Recording finishes when you exit the shell (hit <kbd>Ctrl+D</kbd> or type
`exit`). If the recorded process is not a shell then recording finishes when
the process exits.

If the `filename` argument is given then the resulting recording (called
[asciicast](doc/asciicast-v1.md)) is saved to a local file. It can later be
replayed with `asciinema play <filename>` and/or uploaded to asciinema.org with
`asciinema upload <filename>`. If the `filename` argument is omitted then
(after asking for confirmation) the resulting asciicast is uploaded to
asciinema.org for further playback in a web browser.

`ASCIINEMA_REC=1` is added to recorded process environment variables. This
can be used by your shell's config file (`.bashrc`, `.zshrc`) to alter the
prompt or play a sound when shell is being recorded.

Available options:

* `-c, --command=<command>` - Specify command to record, defaults to $SHELL
* `-t, --title=<title>` - Specify the title of the asciicast
* `-w, --max-wait=<sec>` - Reduce recorded terminal inactivity to max <sec> seconds
* `-y, --yes` - Answer "yes" to all prompts (e.g. upload confirmation)
* `-q, --quiet` - Be quiet, suppress all notices/warnings (implies -y)

### `play <filename>`

__Replay recorded asciicast in a terminal.__

This command replays given asciicast (as recorded by `rec` command) directly in
your terminal.

Playing from a local file:

    asciinema play /path/to/asciicast.json

Playing from HTTP(S) URL:

    asciinema play https://asciinema.org/a/22124.json
    asciinema play http://example.com/demo.json

Playing from asciicast page URL (requires `<link rel="alternate"
type="application/asciicast+json" href="....json">` in page's HTML):

    asciinema play https://asciinema.org/a/22124
    asciinema play http://example.com/blog/post.html

Playing from stdin:

    cat /path/to/asciicast.json | asciinema play -
    ssh user@host cat asciicast.json | asciinema play -

Playing from IPFS:

    asciinema play ipfs:/ipfs/QmcdXYJp6e4zNuimuGeWPwNMHQdxuqWmKx7NhZofQ1nw2V
    asciinema play fs:/ipfs/QmcdXYJp6e4zNuimuGeWPwNMHQdxuqWmKx7NhZofQ1nw2V

Available options:

* `-w, --max-wait=<sec>` - Reduce replayed terminal inactivity to max <sec> seconds
* `-s, --speed=<factor>` - Playback speedup (can be fractional)

NOTE: it is recommended to run `asciinema play` in a terminal of dimensions not
smaller than the one used for recording as there's no "transcoding" of control
sequences for new terminal size.

### `upload <filename>`

__Upload recorded asciicast to asciinema.org site.__

This command uploads given asciicast (as recorded by `rec` command) to
asciinema.org for further playback in a web browser.

`asciinema rec demo.json` + `asciinema play demo.json` + `asciinema upload
demo.json` is a nice combo for when you want to review an asciicast before
publishing it on asciinema.org.

### `auth`

__Manage recordings on asciinema.org account.__

If you want to manage your recordings on asciinema.org (set title/description,
delete etc) you need to authenticate. This command displays the URL you should
open in your web browser to do that.

On every machine you run asciinema recorder, you get a new, unique API token. If
you're already logged in on asciinema.org website and you run `asciinema auth`
from a new computer then this new device will be linked to your account.

You can synchronize your config file (which keeps the API token) across the
machines so all of them use the same token, but that's not necessary. You can
assign new tokens to your account from as many machines as you want.

## Hosting the recordings on the web

As mentioned in the `Usage / rec` section above, if the `filename` argument to
`asciinema rec` is omitted then the resulting asciicast is uploaded
to [asciinema.org](https://asciinema.org) where it's hosted for further playback
in a web browser.

If you prefer to host the recordings yourself, you can do so by recording to a
file (`asciinema rec demo.json`) and using
[asciinema's standalone web player](https://github.com/asciinema/asciinema-player#self-hosting-quick-start)
in your HTML page.

## Configuration file

asciinema uses a config file to keep API token and user settings. In most cases
the location of this file is `$HOME/.config/asciinema/config`.

*NOTE: When you first run asciinema, local API token is generated (UUID) and
saved in the file (unless the file already exists or you have set
`ASCIINEMA_API_TOKEN` environment variable).*

The auto-generated, minimal config file looks like this:

    [api]
    token = <your-api-token-here>

There are several options you can set in this file. Here's a config with all
available options set:

    [api]
    token = <your-api-token-here>
    url = https://asciinema.example.com

    [record]
    command = /bin/bash -l
    maxwait = 2
    yes = true
    quiet = true

    [play]
    maxwait = 1

The options in `[api]` section are related to API location and authentication.
To tell asciinema recorder to use your own asciinema site instance rather than
the default one (asciinema.org), you can set `url` option. API URL can also be
passed via `ASCIINEMA_API_URL` environment variable, as well as API token, via
`ASCIINEMA_API_TOKEN` environment variable.

The options in `[record]` and `[play]` sections have the same meaning as the
options you pass to `asciinema rec`/`asciinema play` command. If you happen to
often use either `-c`, `-w` or `-y` with these commands then consider saving it
as a default in the config file.

*NOTE: If you want to publish your asciinema config file (in public dotfiles
repository) you __should__ remove `token = ...` line from the file and use
`ASCIINEMA_API_TOKEN` environment variable instead.*

### Configuration file locations

In fact, the following locations are checked for the presence of the config
file (in the given order):

* `$ASCIINEMA_CONFIG_HOME/config` - if you have set `$ASCIINEMA_CONFIG_HOME`
* `$XDG_CONFIG_HOME/asciinema/config` - on Linux, `$XDG_CONFIG_HOME` usually points to `$HOME/.config/`
* `$HOME/.config/asciinema/config` - in most cases it's here
* `$HOME/.asciinema/config` - created by asciinema versions prior to 1.1

The first one found is used.

## Contributing

If you want to contribute to this project check out
[Contributing](https://asciinema.org/contributing) page.

## Authors

Developed with passion by [Marcin Kulik](http://ku1ik.com) and great open
source [contributors](https://github.com/asciinema/asciinema/contributors)

## License

Copyright &copy; 2011-2017 Marcin Kulik.

All code is licensed under the GPL, v3 or later. See LICENSE file for details.
