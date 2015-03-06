# asciinema

[![Build Status](https://travis-ci.org/asciinema/asciinema.svg?branch=master)](https://travis-ci.org/asciinema/asciinema)

Terminal session recorder and the best companion of
[asciinema.org](https://asciinema.org).

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
asciinema.org for further playback in the browser.

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

### `upload <filename>`

__Upload recorded asciicast to asciinema.org site.__

This command uploads given asciicast (as recorded by `rec` command) to
asciinema.org for further playback in the browser.

`asciinema rec demo.json` + `asciinema play demo.json` + `asciinema upload
demo.json` is a nice combo for when you want to review an asciicast before
publishing it on asciinema.org.

### `auth`

__Assign local API token to asciinema.org account.__

Every machine you install asciinema recorder on you get a new unique API
token. This command is used to connect this local API token with your
asciinema.org account.

This command displays the URL you should open in your web browser. If you
never logged in to asciinema.org then your account will be automatically
created when opening the URL.

NOTE: it is __necessary__ to do this if you want to __edit or delete__ your
recordings on asciinema.org.

You can synchronize your `~/.asciinema/config` file (which keeps the API
token) across the machines but that's not necessary. You can assign new
tokens to your account from as many machines as you want.

## Contributing

If you want to contribute to this project check out
[Contributing](https://asciinema.org/contributing) page.

## Authors

Developed with passion by [Marcin Kulik](http://ku1ik.com) and great open
source [contributors](https://github.com/asciinema/asciinema/contributors)

## License

Copyright &copy; 2011-2015 Marcin Kulik.

All code is licensed under the GPL, v3 or later. See LICENSE file for details.
