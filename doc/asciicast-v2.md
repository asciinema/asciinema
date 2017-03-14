# asciicast file format (version 2)

asciicast v2 file
is [NDJSON (newline delimited JSON)](https://github.com/ndjson/ndjson-spec) file
where:

* __first line__ contains meta-data (duration, terminal size etc), encoded as JSON
  object,
* __all subsequent lines__ contain stream data, _each line_ representing single stream
  element (event), encoded as JSON array.

## Meta-data

Every asciicast v2 includes the following meta-data:

* `version` - set to 2,
* `width` - terminal width (number of columns),
* `height` - terminal height (number of rows),
* `duration` - total duration of asciicast as floating point number,
* `command` - command that was recorded, as given via `-c` option to `rec`,
* `title` - title of the asciicast, as given via `-t` option to `rec`,
* `env` - map of environment variables useful for debugging playback problems.

Example meta-data line:

    { "version": 2, "width": 80, "height": 24, "duration": 1.515658, "command": "/bin/zsh", "title": null, "env": { "TERM": "xterm-256color", "SHELL": "/bin/zsh" } }

## Stream data

TODO: explain

    [ time, event-type, event-data ]
    
TODO: explain

    [ 1.001376, "o", "Hello world" ]

### "o" event - print to stdout

TODO: change this section to reflect new structure

Frame represents an event of printing new data to terminal's stdout. It is a 2
element array containing **delay** and **data**.

**Delay** is the number of seconds that elapsed since the previous frame (or
since the beginning of the recording in case of the 1st frame) represented as
a floating point number, with microsecond precision.

**Data** is a string containing the data that was printed to a terminal in a
given frame. It has to be valid, UTF-8 encoded JSON string as described in
[JSON RFC section 2.5](http://www.ietf.org/rfc/rfc4627.txt), with all
non-printable Unicode codepoints encoded as `\uXXXX`.

For example, frame `[5.4321, "foo\rbar\u0007..."]` means there was 5 seconds of
inactivity between previous printing and printing of `foo\rbar\u0007...`.

## Complete asciicast v2 example

A very short asciicast v2 file looks like this:

    { "version": 2, "width": 80, "height": 24, "duration": 1.515658, "command": "/bin/zsh", "title": null, "env": { "TERM": "xterm-256color", "SHELL": "/bin/zsh" } }
    [ 0.248848, "o", "\u001b[1;31mHello \u001b[32mWorld!\u001b[0m\n" ]
    [ 1.001376, "o", "This is overwritten\rThis is better." ]
    [ 0.143733, "o", " " ]
    [ 0.541828, "o", "Bye!" ]
