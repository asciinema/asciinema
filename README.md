# asciinema

[![Build Status](https://travis-ci.org/asciinema/asciinema.svg?branch=master)](https://travis-ci.org/asciinema/asciinema)

Terminal session recorder and the best companion of
[asciinema.org](https://asciinema.org).

[![demo](https://asciinema.org/a/17654.png)](https://asciinema.org/a/17654?autoplay=1)

## Installation

On Linux and Mac OS X, __the easiest way to install asciinema__ recorder is to
run the following shell command:

    curl -sL https://asciinema.org/install | sh

[This script](https://asciinema.org/install) will download the latest asciinema
recorder binary for your platform, and install it in your `$PATH`.

Other installation options, including Homebrew and distro packages (Ubuntu,
Fedora, Arch Linux, Gentoo), are [also
available](https://asciinema.org/docs/installation).

If you have Go development environment set up you can `go get
github.com/asciinema/asciinema` to build asciinema and put the binary
in `$GOPATH/bin/asciinema`.

### Building from source

To build asciinema from source you need to have
[Go development environment](http://golang.org/doc/install) set up.

Following the steps below will get the source code and compile it into a single
statically linked binary:

    mkdir -p $GOPATH/src/github.com/asciinema
    git clone https://github.com/asciinema/asciinema.git $GOPATH/src/github.com/asciinema/asciinema
    cd $GOPATH/src/github.com/asciinema/asciinema
    make build

This will produce asciinema binary at `bin/asciinema`.

To install it system wide (to `/usr/local`):

    sudo make install

If you want to install it in other location:

    PREFIX=/the/prefix make install

## Usage

asciinema is composed of multiple commands, similar to `git`, `rails` or
`brew`.

When you run `asciinema` with no arguments help message is displayed showing
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
* `-t, --title=<title>` - Specify title of the asciicast
* `-w, --max-wait=<sec>` - Reduce recorded terminal inactivity to max <sec> seconds
* `-y, --yes` - Answer yes to all prompts (e.g. upload confirmation)

### `play <filename>`

__Replay recorded asciicast in a terminal.__

This command replays given asciicast (as recorded by `rec` command) directly in
your terminal.

NOTE: it is recommended to run it in a terminal of dimensions not smaller than
the one used for recording as there's no "transcoding" of control sequences for
new terminal size.

Available options:

* `-w, --max-wait=<sec>` - Reduce replayed terminal inactivity to max <sec> seconds

### `upload <filename>`

__Upload recorded asciicast to asciinema.org site.__

This command uploads given asciicast (as recorded by `rec` command) to
asciinema.org for further playback in a web browser.

`asciinema rec demo.json` + `asciinema play demo.json` + `asciinema upload
demo.json` is a nice combo for when you want to review an asciicast before
publishing it on asciinema.org.

### `auth`

__Assign local API token to asciinema.org account.__

On every machine you install asciinema recorder, you get a new, unique API
token. This command connects this local token with your asciinema.org account,
and links all asciicasts recorded on this machine with the account.

This command displays the URL you should open in your web browser. If you never
logged in to asciinema.org then your account will be created when opening the
URL.

NOTE: it is __necessary__ to do this if you want to __edit or delete__ your
recordings on asciinema.org.

You can synchronize your config file (which keeps the API token) across the
machines but that's not necessary. You can assign new tokens to your account
from as many machines as you want.

## Configuration file

asciinema uses a config file to keep API token and user settings. In most cases
the location of this file is `$HOME/.config/asciinema/config`.

When you first run `asciinema`, local API token is generated and saved in the
file. It looks like this:

    [api]
    token = d5a2dce4-173f-45b2-a405-ac33d7b70c5f

There are several options you can set in this file. Here's a config with all
available options set:

    [api]
    token = d5a2dce4-173f-45b2-a405-ac33d7b70c5f
    url = https://asciinema.example.com

    [record]
    command = /bin/bash -l
    maxwait = 2
    yes = true

    [play]
    maxwait = 1

The options in `[api]` section are related to API location and authentication.
To tell asciinema recorder to use your own asciinema site instance rather than
the default one (asciinema.org), you can set `url` option. API URL can also be
passed via `ASCIINEMA_API_URL` environment variable.

The options in `[record]` and `[play]` sections have the same meaning as the
options you pass to `asciinema rec`/`asciinema play` command. If you happen to
often use either `-c`, `-w` or `-y` with these commands then consider saving it
as a default in the config file.

### Configuration file locations

In fact, the following locations are checked for the presence of the config
file (in the given order):

* `$ASCIINEMA_CONFIG_HOME/config` - if you have set `$ASCIINEMA_CONFIG_HOME`
* `$XDG_CONFIG_HOME/asciinema/config` - on Linux, `$XDG_CONFIG_HOME` usually points to `$HOME/.config/`
* `$HOME/.config/asciinema/config` - in most cases it's here
* `$HOME/.asciinema/config` - created by asciinema versions prior to 1.1

The first one which is found is used.

## Contributing

If you want to contribute to this project check out
[Contributing](https://asciinema.org/contributing) page.

## Authors

Developed with passion by [Marcin Kulik](http://ku1ik.com) and great open
source [contributors](https://github.com/asciinema/asciinema/contributors)

## License

Copyright &copy; 2011-2015 Marcin Kulik.

All code is licensed under the GPL, v3 or later. See LICENSE file for details.
