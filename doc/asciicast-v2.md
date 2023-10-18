# asciicast file format (version 2)

asciicast v2 file is [newline-delimited JSON](http://jsonlines.org/) file where:

* __first line__ contains header (initial terminal size, timestamp and other
  meta-data), encoded as JSON object,
* __all following lines__ form an event stream, _each line_ representing a
  separate event, encoded as 3-element JSON array.

Example file:

```json
{"version": 2, "width": 80, "height": 24, "timestamp": 1504467315, "title": "Demo", "env": {"TERM": "xterm-256color", "SHELL": "/bin/zsh"}}
[0.248848, "o", "\u001b[1;31mHello \u001b[32mWorld!\u001b[0m\n"]
[1.001376, "o", "That was ok\rThis is better."]
[1.500000, "m", ""]
[2.143733, "o", "Now... "]
[4.050000, "r", "80x24"]
[6.541828, "o", "Bye!"]
```

Suggested file extension is `.cast`, suggested media type is
`application/x-asciicast`.

## Header

asciicast header is JSON-encoded object containing recording meta-data.

### Required header attributes:

#### `version`

Must be set to `2`. Integer.

#### `width`

Initial terminal width (number of columns). Integer.

#### `height`

Initial terminal height (number of rows). Integer.

### Optional header attributes:

#### `timestamp`

Unix timestamp of the beginning of the recording session. Integer.

#### `duration`

Duration of the whole recording in seconds (when it's known upfront). Float.

#### `idle_time_limit`

Idle time limit, as given via `-i` option to `asciinema rec`. Float.

This should be used by an asciicast player to reduce all terminal inactivity
(delays between frames) to maximum of `idle_time_limit` value.

#### `command`

Command that was recorded, as given via `-c` option to `asciinema rec`. String.

#### `title`

Title of the asciicast, as given via `-t` option to `asciinema rec`. String.

#### `env`

Map of captured environment variables. Object (String -> String).

Example env:

```json
"env": {
  "SHELL": "/bin/bash",
  "TERM": "xterm-256color"
}
```

> Official asciinema recorder captures only `SHELL` and `TERM` by default. All
> implementations of asciicast-compatible terminal recorder should not capture
> any additional environment variables unless explicitly permitted by the user.

#### `theme`

Color theme of the recorded terminal. Object, with the following attributes:

- `fg` - normal text color,
- `bg` - normal background color,
- `palette` - list of 8 or 16 colors, separated by colon character.

All colors are in the CSS `#rrggbb` format.

Example theme:

```json
"theme": {
  "fg": "#d0d0d0",
  "bg": "#212121",
  "palette": "#151515:#ac4142:#7e8e50:#e5b567:#6c99bb:#9f4e85:#7dd6cf:#d0d0d0:#505050:#ac4142:#7e8e50:#e5b567:#6c99bb:#9f4e85:#7dd6cf:#f5f5f5"
}
```

> A specific technique of obtaining the colors from a terminal (using xrdb,
> requesting them from a terminal via special escape sequences etc) doesn't
> matter as long as the recorder can save it in the above format.

## Event stream

Each element of the event stream is a 3-tuple encoded as JSON array:

    [time, code, data]

Where:

* `time` (float) - indicates when the event happened, represented as the number
  of seconds since the beginning of the recording session,
* `code` (string) - specifies type of event, one of: `"o"`, `"i"`, `"m"`, `"r"`
* `data` (any) - event specific data, described separately for each event
  code.

For example, let's look at the following line:

    [1.001376, "o", "Hello world"]

It represents:

* output (code `o`),
* of text `Hello world`,
* which happened `1.001376` sec after the start of the recording session.

### Supported event codes

This section describes the event codes supported in asciicast v2 format.

The list is open to extension, and new event codes may be added in both the
current and future versions of the format. For example, we may add new event
code for text overlay (subtitles display).

A tool which interprets the event stream (web/cli player, post-processor) should
ignore (or pass through) event codes it doesn't understand or doesn't care
about.

#### "o" - output, data written to a terminal

Event of code `"o"` represents printing new data to a terminal.

`data` is a string containing the data that was printed. It must be valid, UTF-8
encoded JSON string as described in [JSON RFC section
2.5](http://www.ietf.org/rfc/rfc4627.txt), with any non-printable Unicode
codepoints encoded as `\uXXXX`.

#### "i" - input, data read from a terminal

Event of code `"i"` represents character typed in by the user, or more
specifically, raw data sent from a terminal emulator to stdin of the recorded
program (usually shell).

`data` is a string containing captured ASCII character representing a key, or a
control character like `"\r"` (enter), `"\u0001"` (ctrl-a), `"\u0003"` (ctrl-c),
etc. Like with `"o"` event, it's UTF-8 encoded JSON string, with any
non-printable Unicode codepoints encoded as `\uXXXX`.

> Official asciinema recorder doesn't capture keyboard input by default. All
> implementations of asciicast-compatible terminal recorder should not capture
> it either unless explicitly requested by the user.

#### "m" - marker

Event of code `"m"` represents a marker.

When marker is encountered in the event stream and "pause on markers"
functionality of the player is enabled, the playback should pause, and wait for
the user to resume.

`data` can be used to annotate a marker. Annotations may be used to e.g.
show a list of named "chapters".

#### "r" - resize

Event of code `"r"` represents terminal resize.

Those are captured in response to `SIGWINCH` signal.

`data` contains new terminal size (columns + rows) formatted as
`"{COLS}x{ROWS}"`, e.g. `"80x24"`.

## Notes on compatibility

Version 2 of asciicast file format solves several problems which couldn't be
easily fixed in the old format:

* minimal memory usage when recording and replaying arbitrarily long sessions -
  disk space is the only limit,
* when the recording session is interrupted (computer crash, accidental close of
  terminal window) you don't lose the whole recording,
* it's real-time streaming friendly.

Due to file structure change (standard JSON => newline-delimited JSON) version 2
is not backwards compatible with version 1. Support for v2 has been added in:

* [asciinema terminal recorder](https://github.com/asciinema/asciinema) - 2.0.0
* [asciinema web player](https://github.com/asciinema/asciinema-player) - 2.6.0
* [asciinema server](https://github.com/asciinema/asciinema-server) - v20171105
  tag in git repository
