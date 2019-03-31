_Note: This is README for `development` branch. [See the version for latest stable release](https://github.com/asciinema/asciinema/blob/master/README.md)._

# asciinema

[![Build Status](https://travis-ci.org/asciinema/asciinema.svg?branch=develop)](https://travis-ci.org/asciinema/asciinema)
[![PyPI](https://img.shields.io/pypi/v/asciinema.svg)](https://pypi.org/project/asciinema/)
[![license](http://img.shields.io/badge/license-GNU-blue.svg)](https://raw.githubusercontent.com/asciinema/asciinema/master/LICENSE)

Terminal session recorder and the best companion of
[asciinema.org](https://asciinema.org).

[![demo](https://asciinema.org/a/113463.svg)](https://asciinema.org/a/113463?autoplay=1)

## Quick intro

asciinema lets you easily record terminal sessions and replay
them in a terminal as well as in a web browser.

Install latest version ([other installation options](#installation)):

    sudo pip3 install asciinema

Record your first session:

    asciinema rec first.cast

Now replay it with double speed:

    asciinema play -s 2 first.cast

Or with normal speed but with idle time limited to 2 seconds:

    asciinema play -i 2 first.cast

You can pass `-i 2` to `asciinema rec` as well, to set it permanently on a
recording. Idle time limiting makes the recordings much more interesting to
watch. Try it.

If you want to watch and share it on the web, upload it:

    asciinema upload first.cast

The above uploads it to [asciinema.org](https://asciinema.org), which is a
default [asciinema-server](https://github.com/asciinema/asciinema-server)
instance, and prints a secret link you can use to watch your recording in a web
browser.

You can record and upload in one step by omitting the filename:

    asciinema rec

You'll be asked to confirm the upload when the recording is done. Nothing is
sent anywhere without your consent.

These are the basics, but there's much more you can do. The following sections
cover installation, usage and hosting of the recordings in more detail.

## Installation

### Python package

asciinema is available on [PyPI](https://pypi.python.org/pypi/asciinema) and can
be installed with pip (Python 3 with setuptools required):

    sudo pip3 install asciinema

This is the recommended way of installation, which gives you the latest released
version.

### Native packages

asciinema is included in repositories of most popular package managers on Mac OS
X, Linux and FreeBSD. Look for package named `asciinema`. See the
[list of available packages](https://asciinema.org/docs/installation).

### Running latest version from source code checkout

If you can't use Python package or native package for your OS is outdated you
can clone the repo and run asciinema straight from the checkout.

Clone the repo:

    git clone https://github.com/asciinema/asciinema.git
    cd asciinema

If you want latest stable version:

    git checkout master

If you want current development version:

    git checkout develop

Then run it with:

    python3 -m asciinema --version

### Docker image

asciinema Docker image is based on Ubuntu 18.04 and has the latest version of
asciinema recorder pre-installed.

    docker pull asciinema/asciinema

When running it don't forget to allocate a pseudo-TTY (`-t`), keep STDIN open
(`-i`) and mount config directory volume (`-v`):

    docker run --rm -ti -v "$HOME/.config/asciinema":/root/.config/asciinema asciinema/asciinema rec

Container's entrypoint is set to `/usr/local/bin/asciinema` so you can run the
container with any arguments you would normally pass to `asciinema` binary (see
Usage section for commands and options).

There's not much software installed in this image though. In most cases you may
want to install extra programs before recording. One option is to derive new
image from this one (start your custom Dockerfile with `FROM
asciinema/asciinema`). Another option is to start the container with `/bin/bash`
as the entrypoint, install extra packages and manually start `asciinema rec`:

    docker run --rm -ti -v "$HOME/.config/asciinema":/root/.config/asciinema --entrypoint=/bin/bash asciinema/asciinema
    root@6689517d99a1:~# apt-get install foobar
    root@6689517d99a1:~# asciinema rec

## Usage

asciinema is composed of multiple commands, similar to `git`, `apt-get` or
`brew`.

When you run `asciinema` with no arguments help message is displayed, listing
all available commands with their options.

### `rec [filename]`

__Record terminal session.__

By running `asciinema rec [filename]` you start a new recording session. The
command (process) that is recorded can be specified with `-c` option (see
below), and defaults to `$SHELL` which is what you want in most cases.

You can temporarily pause recording of terminal by pressing <kbd>Ctrl+P</kbd>.
This is useful when you want to execute some commands during the recording
session that should not be captured (e.g. pasting secrets). Resume by pressing
<kbd>Ctrl+P</kbd> again.

Recording finishes when you exit the shell (hit <kbd>Ctrl+D</kbd> or type
`exit`). If the recorded process is not a shell then recording finishes when
the process exits.

If the `filename` argument is omitted then (after asking for confirmation) the
resulting asciicast is uploaded to
[asciinema-server](https://github.com/asciinema/asciinema-server) (by default to
asciinema.org), where it can be watched and shared.

If the `filename` argument is given then the resulting recording (called
[asciicast](doc/asciicast-v2.md)) is saved to a local file. It can later be
replayed with `asciinema play <filename>` and/or uploaded to asciinema server
with `asciinema upload <filename>`.

`ASCIINEMA_REC=1` is added to recorded process environment variables. This
can be used by your shell's config file (`.bashrc`, `.zshrc`) to alter the
prompt or play a sound when the shell is being recorded.

Available options:

* `--stdin` - Enable stdin (keyboard) recording (see below)
* `--append` - Append to existing recording
* `--raw` - Save raw STDOUT output, without timing information or other metadata
* `--overwrite` - Overwrite the recording if it already exists
* `-c, --command=<command>` - Specify command to record, defaults to $SHELL
* `-e, --env=<var-names>` - List of environment variables to capture, defaults
  to `SHELL,TERM`
* `-t, --title=<title>` - Specify the title of the asciicast
* `-i, --idle-time-limit=<sec>` - Limit recorded terminal inactivity to max `<sec>` seconds
* `-y, --yes` - Answer "yes" to all prompts (e.g. upload confirmation)
* `-q, --quiet` - Be quiet, suppress all notices/warnings (implies -y)

Stdin recording allows for capturing of all characters typed in by the user in
the currently recorded shell. This may be used by a player (e.g.
[asciinema-player](https://github.com/asciinema/asciinema-player)) to display
pressed keys. Because it's basically a key-logging (scoped to a single shell
instance), it's disabled by default, and has to be explicitly enabled via
`--stdin` option.

### `play <filename>`

__Replay recorded asciicast in a terminal.__

This command replays given asciicast (as recorded by `rec` command) directly in
your terminal.

Following keyboard shortcuts are available:

- <kbd>Space</kbd> - toggle pause,
- <kbd>.</kbd> - step through a recording a frame at a time (when paused),
- <kbd>Ctrl+C</kbd> - exit.

Playing from a local file:

    asciinema play /path/to/asciicast.cast

Playing from HTTP(S) URL:

    asciinema play https://asciinema.org/a/22124.cast
    asciinema play http://example.com/demo.cast

Playing from asciicast page URL (requires `<link rel="alternate"
type="application/x-asciicast" href="/my/ascii.cast">` in page's HTML):

    asciinema play https://asciinema.org/a/22124
    asciinema play http://example.com/blog/post.html

Playing from stdin:

    cat /path/to/asciicast.cast | asciinema play -
    ssh user@host cat asciicast.cast | asciinema play -

Playing from IPFS:

    asciinema play dweb:/ipfs/QmNe7FsYaHc9SaDEAEXbaagAzNw9cH7YbzN4xV7jV1MCzK/ascii.cast

Available options:

* `-i, --idle-time-limit=<sec>` - Limit replayed terminal inactivity to max `<sec>` seconds
* `-s, --speed=<factor>` - Playback speed (can be fractional)

> For the best playback experience it is recommended to run `asciinema play` in
> a terminal of dimensions not smaller than the one used for recording, as
> there's no "transcoding" of control sequences for new terminal size.

### `cat <filename>`

__Print full output of recorded asciicast to a terminal.__

While `asciinema play <filename>` replays the recorded session using timing
information saved in the asciicast, `asciinema cat <filename>` dumps the full
output (including all escape sequences) to a terminal immediately.

`asciinema cat existing.cast >output.txt` gives the same result as recording via
`asciinema rec --raw output.txt`.

### `upload <filename>`

__Upload recorded asciicast to asciinema.org site.__

This command uploads given asciicast (recorded by `rec` command) to
asciinema.org, where it can be watched and shared.

`asciinema rec demo.cast` + `asciinema play demo.cast` + `asciinema upload
demo.cast` is a nice combo if you want to review an asciicast before
publishing it on asciinema.org.

### `auth`

__Link your install ID with your asciinema.org user account.__

If you want to manage your recordings (change title/theme, delete) at
asciinema.org you need to link your "install ID" with asciinema.org user
account.

This command displays the URL to open in a web browser to do that. You may be
asked to log in first.

Install ID is a random ID ([UUID
v4](https://en.wikipedia.org/wiki/Universally_unique_identifier)) generated
locally when you run asciinema for the first time, and saved at
`$HOME/.config/asciinema/install-id`. Its purpose is to connect local machine
with uploaded recordings, so they can later be associated with asciinema.org
account. This way we decouple uploading from account creation, allowing them to
happen in any order.

> A new install ID is generated on each machine and system user account you use
> asciinema on, so in order to keep all recordings under a single asciinema.org
> account you need to run `asciinema auth` on all of those machines.

> asciinema versions prior to 2.0 confusingly referred to install ID as "API
> token".

## Hosting the recordings on the web

As mentioned in the `Usage > rec` section above, if the `filename` argument to
`asciinema rec` is omitted then the recorded asciicast is uploaded to
[asciinema.org](https://asciinema.org). You can watch it there and share it via
secret URL.

If you prefer to host the recordings yourself, you can do so by either:

- recording to a file (`asciinema rec demo.cast`), and using [asciinema's
  standalone web
  player](https://github.com/asciinema/asciinema-player#self-hosting-quick-start)
  in your HTML page, or
- setting up your own
  [asciinema-server](https://github.com/asciinema/asciinema-server) instance,
  and [setting API URL
  accordingly](https://github.com/asciinema/asciinema-server/blob/master/docs/INSTALL.md#using-asciinema-recorder-with-your-instance).

## Configuration file

You can configure asciinema by creating config file at
`$HOME/.config/asciinema/config`.

Configuration is split into sections (`[api]`, `[record]`, `[play]`). Here's a
list of all available options for each section:

```ini
[api]

; API server URL, default: https://asciinema.org
; If you run your own instance of asciinema-server then set its address here
; It can also be overriden by setting ASCIINEMA_API_URL environment variable
url = https://asciinema.example.com

[record]

; Command to record, default: $SHELL
command = /bin/bash -l

; Enable stdin (keyboard) recording, default: no
stdin = yes

; List of environment variables to capture, default: SHELL,TERM
env = SHELL,TERM,USER

; Limit recorded terminal inactivity to max n seconds, default: off
idle_time_limit = 2

; Answer "yes" to all interactive prompts, default: no
yes = true

; Be quiet, suppress all notices/warnings, default: no
quiet = true

[play]

; Playback speed (can be fractional), default: 1
speed = 2

; Limit replayed terminal inactivity to max n seconds, default: off
idle_time_limit = 1

[notifications]

; Should desktop notifications be enabled, default: yes
enabled = no

; Custom notification command
; Environment variable $TEXT contains notification text
command = tmux display-message "$TEXT"
```

A very minimal config file could look like that:

```ini
[record]
idle_time_limit = 2
```

Config directory location can be changed by setting `$ASCIINEMA_CONFIG_HOME`
environment variable.

If `$XDG_CONFIG_HOME` is set on Linux then asciinema uses
`$XDG_CONFIG_HOME/asciinema` instead of `$HOME/.config/asciinema`.

> asciinema versions prior to 1.1 used `$HOME/.asciinema`. If you have it
> there you should `mv $HOME/.asciinema $HOME/.config/asciinema`.

## Contributing

If you want to contribute to this project check out
[Contributing](https://asciinema.org/contributing) page.

## Authors

Developed with passion by [Marcin Kulik](http://ku1ik.com) and great open
source [contributors](https://github.com/asciinema/asciinema/contributors).

## License

Copyright &copy; 2011–2019 Marcin Kulik.

All code is licensed under the GPL, v3 or later. See LICENSE file for details.
