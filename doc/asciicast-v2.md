# asciicast file format (version 2)

asciicast v2 file
is [NDJSON (newline delimited JSON)](https://github.com/ndjson/ndjson-spec) file
where:

* __first line__ contains meta-data (duration, initial terminal size, etc),
  encoded as JSON object,
* __all subsequent lines__ form an event stream, _each line_ representing a
  separate event stream item, encoded as JSON array.

## Meta-data

The following meta-data is **required** in asciicast v2:

* `version` - set to 2,
* `width` - initial terminal width (number of columns),
* `height` - initial terminal height (number of rows).

The following meta-data is **optional** in asciicast v2:

* `command` - command that was recorded, as given via `-c` option to `asciinema rec`,
* `title` - title of the asciicast, as given via `-t` option to `asciinema rec`,
* `env` - map of environment variables useful for debugging playback problems.

Example meta-data line:

    { "version": 2, "width": 80, "height": 24, "command": "/bin/zsh", "title": null, "env": { "TERM": "xterm-256color", "SHELL": "/bin/zsh" } }

## Event stream

Each element of the event stream is a 3-tuple encoded as JSON array:

    [ time, event-type, event-data ]

Where:

* `time` (number) - relative time (in seconds) since the previous event occured
  (or since the beginning of the recording session for the first event in the
  stream),
* `event-type` (string) - one of: `"o"`, `"i"`, `"size"`,
* `event-data` (any) - event specific data, described separately for each event
  type.

For example, let's look at the following line:

    [ 1.001376, "o", "Hello world" ]

It represents the event which:

* happened 1.001376 sec after a previous event in the stream,
* is of type `"o"` (print to stdout, see below),
* has data `"Hello world"`.

### Supported event types

This section describes the event types supported in asciicast v2 format.

The list is open to extension, and new event types may be added in both the
current and future versions of the format. For example, we may add new event
type for text overlay (subtitles display).

A tool which interprets the event stream (web/cli player, post-processor) should
ignore event types it doesn't understand or doesn't care about. However, it
should honor the amount of time specified by such stream item (by pausing for
this amount of time when playing back) in order to keep the subsequent events on
the timeline.

#### "o" - output, printing to stdout

Event of type `"o"` represents printing new data to terminal's stdout.

`event-data` is a string containing the data that was printed to a terminal. It
has to be valid, UTF-8 encoded JSON string as described
in [JSON RFC section 2.5](http://www.ietf.org/rfc/rfc4627.txt), with all
non-printable Unicode codepoints encoded as `\uXXXX`.

#### "i" - input, from keyboard (planned?)

TODO

not supported by current versions of the recorder and players

#### "size" - terminal resize (planned?)

TODO

not supported by current versions of the recorder and players

## Complete asciicast v2 example

A very short asciicast v2 file looks like this:

    { "version": 2, "width": 80, "height": 24, "duration": 1.515658, "command": "/bin/zsh", "title": null, "env": { "TERM": "xterm-256color", "SHELL": "/bin/zsh" } }
    [ 0.248848, "o", "\u001b[1;31mHello \u001b[32mWorld!\u001b[0m\n" ]
    [ 1.001376, "o", "This is overwritten\rThis is better." ]
    [ 0.143733, "o", " " ]
    [ 0.541828, "o", "Bye!" ]

The final `"Bye!"` was printed to a terminal after 1.935785 sec (0.248848 +
1.001376 + 0.143733 + 0.541828).
