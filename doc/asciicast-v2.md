# asciicast file format (version 2)

asciicast v2 file
is [NDJSON (newline delimited JSON)](https://github.com/ndjson/ndjson-spec) file
where:

* __first line__ contains header (initial terminal size, timestamp and other
  meta-data), encoded as JSON object,
* __all subsequent lines__ form an event stream, _each line_ representing a
  separate event stream item, encoded as JSON array.

By making the event stream a first class concept in the v2 format we get the
following benefits:

* it enables live, incremental writing to a file during recording (with v1
  format the final recording JSON can only be written as a whole after finishing
  the recording session),
* it allows the players to start the playback as soon as they read the meta-data
  line (contrary to v1 format which requires reading the whole file),
* whether you're recording to a file or streaming via UNIX pipe or WebSocket the
  data representation is the same.

## Header

asciicast header is JSON-encoded object containing recording meta-data.

Example header:

```json
{"version": 2, "width": 80, "height": 24, "timestamp": 1504467315, "title": "Vim tutorial #1", "env": {"TERM": "xterm-256color", "SHELL": "/bin/zsh"}}
```

### Required header attributes:

#### `version`

Must be set to `2`.

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
  "SHELL": "xterm-256color",
  "TERM": "/bin/bash"
}
```

Official asciinema recorder captures only `SHELL` and `TERM` by default. All
implementations of asciicast-compatible terminal recorder should not capture any
additional environment variables unless explicitly permitted by the user.

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

A specific technique of obtaining the colors from a terminal (using xrdb,
requesting them from a terminal via special escape sequences etc) doesn't matter
as long as the recorder can save it in the above format.

## Event stream

Each element of the event stream is a 3-tuple encoded as JSON array:

    [time, event-type, event-data]

Where:

* `time` (float) - indicates when this event happened, represented as the number
  of seconds since the beginning of the recording session,
* `event-type` (string) - one of: `"o"`, `"i"`, `"size"`,
* `event-data` (any) - event specific data, described separately for each event
  type.

For example, let's look at the following line:

    [1.001376, "o", "Hello world"]

It represents the event which:

* happened 1.001376 sec after the start of the recording session,
* is of type `"o"` (print to stdout, see below),
* has data `"Hello world"`.

### Supported event types

This section describes the event types supported in asciicast v2 format.

The list is open to extension, and new event types may be added in both the
current and future versions of the format. For example, we may add new event
type for text overlay (subtitles display).

A tool which interprets the event stream (web/cli player, post-processor) should
ignore (or pass through) event types it doesn't understand or doesn't care
about.

#### "o" - data written to stdout

Event of type `"o"` represents printing new data to terminal's stdout.

`event-data` is a string containing the data that was printed to a terminal. It
has to be valid, UTF-8 encoded JSON string as described
in [JSON RFC section 2.5](http://www.ietf.org/rfc/rfc4627.txt), with all
non-printable Unicode codepoints encoded as `\uXXXX`.

#### "i" - data read from stdin

Event of type `"i"` represents character(s) typed in (or pasted) by the user, or
more specifically, data sent from terminal emulator to stdin of the recorded
shell.

`event-data` is a string containing the captured character(s). Like with `"o"`
event, it's UTF-8 encoded JSON string, with all non-printable Unicode codepoints
encoded as `\uXXXX`.

_Note: Official asciinema recorder doesn't capture stdin by default. All
implementations of asciicast-compatible terminal recorder should not capture it
either unless explicitly permitted by the user._

#### "size" - terminal resize (planned?)

TODO

not supported by current versions of the recorder and players

## Complete asciicast v2 example

A very short asciicast v2 file looks like this:

    {"version": 2, "width": 80, "height": 24, "timestamp": 1504467315, "title": "Demo", "env": {"TERM": "xterm-256color", "SHELL": "/bin/zsh"}}
    [0.248848, "o", "\u001b[1;31mHello \u001b[32mWorld!\u001b[0m\n"]
    [1.001376, "o", "This is overwritten\rThis is better."]
    [2.143733, "o", " "]
    [6.541828, "o", "Bye!"]
