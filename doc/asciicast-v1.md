# asciicast file format (version 1)

asciicast file is JSON file containing meta-data like duration or title of the
recording, and the actual content printed to terminal's stdout during
recording.

Version 1 of the format was used by the asciinema recorder versions 1.0 up to 1.4.

## Attributes

Every asciicast includes the following set of attributes:

* `version` - set to 1,
* `width` - terminal width (number of columns),
* `height` - terminal height (number of rows),
* `duration` - total duration of asciicast as floating point number,
* `command` - command that was recorded, as given via `-c` option to `rec`,
* `title` - title of the asciicast, as given via `-t` option to `rec`,
* `env` - map of environment variables useful for debugging playback problems,
* `stdout` - array of "frames", see below.

### Frame

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

## Example asciicast

A very short asciicast may look like this:

    {
      "version": 1,
      "width": 80,
      "height": 24,
      "duration": 1.515658,
      "command": "/bin/zsh",
      "title": "",
      "env": {
        "TERM": "xterm-256color",
        "SHELL": "/bin/zsh"
      },
      "stdout": [
        [
          0.248848,
          "\u001b[1;31mHello \u001b[32mWorld!\u001b[0m\n"
        ],
        [
          1.001376,
          "I am \rThis is on the next line."
        ]
      ]
    }
