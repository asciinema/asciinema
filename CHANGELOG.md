# asciinema changelog

## 3.0.0 (TBD)

This is a complete rewrite of asciinema in Rust, upgrading the recording file
format, introducing terminal live streaming, and bringing numerous improvements
across the board.

### New features

* New `stream` command for terminal live streaming, providing local mode
  (built-in HTTP server) and remote mode (relaying via asciinema server, more
  about it [here](https://docs.asciinema.org/manual/server/streaming/))
* New `session` command for simultaneous recording and streaming
* New `convert` command for format conversion between asciicast versions or
  exporting to plain text log / raw output
* New [asciicast v3 file
  format](https://docs.asciinema.org/manual/asciicast/v3/) as the new default
  output format
* Terminal theme capture - terminal colors are now automatically captured and
  saved in recordings
* New output format - plain text log - use `.txt` extension for the output
  filename or select explicitly with `--output-format txt`
* `rec`: New `--return` option for propagating session exit status
* `rec`: Session exit status is now saved as "x" (exit) event in asciicast v3
  recordings
* `rec`: Parent directories are automatically created when recording to
  non-existing paths
* `rec`: Terminal version (XTVERSION OSC query) is now saved in asciicast
  header as `term.version`
* `rec`: New `--headless` option for forcing headless mode
* `rec`: New `--log-file` option for enabling logging, e.g. for troubleshooting
  I/O errors
* `play`: New `--resize` option for enabling terminal auto-resize during playback (terminal support varies)
* New `--server-url` option for setting custom server URL (for self-hosted
  servers), as an alternative to config file and `ASCIINEMA_SERVER_URL`
  environment variable
* New system-wide configuration file `/etc/asciinema/config.toml` for setting
  defaults for all users
* Command name prefix matching - you can use `asciinema r` instead of
  `asciinema rec`, `asciinema u` instead of `asciinema upload`, etc.
* tmux status bar is now used for notifications when tmux session is detected
  and no other desktop notification mechanism is available

### Improvements

* Prompt for setting up default asciinema server URL on first use, unless one
  is configured upfront
* Comprehensive `--help` messages (vs concise `-h`), with examples for each
  subcommand
* Complete set of man pages and shell auto-completion files can be generated
  during build (see README.md)
* `rec`: Fixed saving of custom record command (`--command`) value in asciicast
  header
* `rec`: `--append` option can now be used with `--format raw`
* `upload`: Recording file is validated for correctness/formatting before upload
* Better error message when non-UTF-8 locale is detected


### Breaking changes

* `rec`: Filename argument is now required, use explicit `upload` command for
  publishing a local recording
* `rec`: Default output format is now asciicast v3 instead of v2 - asciinema
  server and player support it already, but be aware of this if you're using
  custom tooling - use `--output-format asciicast-v2` for backward compatibility
* `rec`: `--stdin` option has been renamed to `--capture-input` / `-I` for
  clarity
* `rec`: `--env` option has been renamed to `--capture-env`, short `-e` variant
  has been removed
* `rec`: `--cols` and `--rows` options have been replaced with single
  `--window-size COLSxROWS` option
* `rec`: `--raw` option has been removed, superceded by `--output-format raw`
* `rec`: `--yes` / `-y` option has been removed since there's no upload
  confirmation anymore
* `rec`: Using both `--append` and `--overwrite` options together now produces
  an immediate error instead of silently ignoring one option
* `cat`: This command now concatenates multiple recordings instead of dumping
  raw output - use `convert --output-format raw` for 2.x behavior
* `play`: `--out-fmt` and `--stream` options have been removed
* User configuration file changed format from "ini-style" to TOML, and moved
  from `~/.config/asciinema/config` to `~/.config/asciinema/config.toml` - check
  [configuration docs](https://docs.asciinema.org/manual/cli/configuration/) for
  details
* Removed built-in support for desktop notifications via terminal-notifier in
  favor of AppleScript on macOS

### Other changes

* Install ID location changed from `XDG_CONFIG_HOME/asciinema/install-id`
  (`$HOME/.config/asciinema`) to `XDG_STATE_HOME/asciinema/install-id`
  (`$HOME/.local/state/asciinema`) - for backward compatibility the previous
  location is still used if the file already exists there
* `ASCIINEMA_REC` environment variable, which was set to `1` for sessions
  started with `asciinema rec`, has been superceded by `ASCIINEMA_SESSION`, which
  is set to a unique session ID by `rec`, `stream` and `session` commands - the
  original `ASCIINEMA_REC=1` is still set by `rec` command for backward
  compatibility
* `ASCIINEMA_API_URL` environment variable has been superceded by
  `ASCIINEMA_SERVER_URL` for setting custom server URL - the original
  `ASCIINEMA_API_URL` still works but is deprecated

## 2.4.0 (2023-10-23)

* When recording without file arg we now ask whether to save, upload or discard the recording (#576)
* Added capture of terminal resize events (#565)
* Fixed blocking write error when PTY master is not ready (#569) (thanks @Low-power!)
* Fixed "broken pipe" errors when piping certain commands during recording (#369) (thanks @Low-power!)
* Fixed crash during playback of cast files with trailing blank line (#577)

## 2.3.0 (2023-07-05)

* Added official support for Python 3.11
* Dropped official support for Python 3.6
* Implemented markers in `rec` and `play -m` commands
* Added `--loop` option for looped playback in `play` command
* Added `--stream` and `--out-fmt` option for customizing output of `play` command
* Improved terminal charset detection (thanks @djds)
* Extended `cat` command to support multiple files (thanks @Low-power)
* Improved upload error messages
* Fixed direct playback from URL
* Made raw output start with terminal size sequence (`\e[8;H;Wt`)
* Prevented recording to stdout when it's a TTY
* Added target file permission checks to avoid ugly errors
* Removed named pipe re-opening, which was causing hangs in certain scenarios
* Improved PTY/TTY data reading - it goes in bigger chunks now (256 kb)
* Fixed deadlock in PTY writes (thanks @Low-power)
* Improved input forwarding from stdin
* Ignored OSC responses in recorded stdin stream

## 2.2.0 (2022-05-07)

* Added official support for Python 3.8, 3.9, 3.10
* Dropped official support for Python 3.5
* Added `--cols` / `--rows` options for overriding size of pseudo-terminal reported to recorded program
* Improved behaviour of `--append` when output file doesn't exist
* Keyboard input is now explicitly read from a TTY device in addition to stdin (when stdin != TTY)
* Recorded program output is now explicitly written to a TTY device instead of stdout
* Dash char (`-`) can now be passed as output filename to write asciicast to stdout
* Diagnostic messages are now printed to stderr (without colors when stderr != TTY)
* Improved robustness of writing asciicast to named pipes
* Lots of codebase modernizations (many thanks to Davis @djds Schirmer!)
* Many other internal refactorings

## 2.1.0 (2021-10-02)

* Ability to pause/resume terminal capture with `C-\` key shortcut
* Desktop notifications - only for the above pause feature at the moment
* Removed dependency on tput/ncurses (thanks @arp242 / Martin Tournoij!)
* ASCIINEMA_REC env var is back (thanks @landonb / Landon Bouma!)
* Terminal answerbacks (CSI 6 n) in `asciinema cat` are now hidden (thanks @djpohly / Devin J. Pohly!)
* Codeset detection works on HP-UX now (thanks @michael-o / Michael Osipov!)
* Attempt at recording to existing file suggests use of `--overwrite` option now
* Upload for users with very long `$USER` is fixed
* Added official support for Python 3.8 and 3.9
* Dropped official support for EOL-ed Python 3.4 and 3.5

## 2.0.2 (2019-01-12)

* Official support for Python 3.7
* Recording is now possible on US-ASCII locale (thanks Jean-Philippe @jpouellet Ouellet!)
* Improved Android support (thanks Fredrik @fornwall Fornwall!)
* Possibility of programatic recording with `asciinema.record_asciicast` function
* Uses new JSON response format added recently to asciinema-server
* Tweaked message about how to stop recording (thanks Bachynin @vanyakosmos Ivan!)
* Added proper description and other metadata to Python package (thanks @Crestwave!)

## 2.0.1 (2018-04-04)

* Fixed example in asciicast v2 format doc (thanks Josh "@anowlcalledjosh" Holland!)
* Replaced deprecated `encodestring` (since Python 3.1) with `encodebytes` (thanks @delirious-lettuce!)
* Fixed location of config dir (you can `mv ~/.asciinema ~/.config/asciinema`)
* Internal refactorings

## 2.0 (2018-02-10)

This major release brings many new features, improvements and bugfixes. The most
notable ones:

* new [asciicast v2 file format](doc/asciicast-v2.md)
* recording and playback of arbitrarily long session with minimal memory usage
* ability to live-stream via UNIX pipe: `asciinema rec unix.pipe` + `asciinema play unix.pipe` in second terminal tab/window
* optional stdin recording (`asciinema rec --stdin`)
* appending to existing recording (`asciinema rec --append <filename>`)
* raw recording mode, storing only stdout bytes (`asciinema rec --raw <filename>`)
* environment variable white-listing (`asciinema rec --env="VAR1,VAR2..."`)
* toggling pause in `asciinema play` by <kbd>Space</kbd>
* stepping through a recording one frame at a time with <kbd>.</kbd> (when playback paused)
* new `asciinema cat <filename>` command to dump full output of the recording
* playback from new IPFS URL scheme: `dweb:/ipfs/` (replaces `fs:/`)
* lots of other bugfixes and improvements
* dropped official support for Python 3.3 (although it still works on 3.3)

## 1.4.0 (2017-04-11)

* Dropped distutils fallback in setup.py - setuptools required now (thanks Jakub "@jakubjedelsky" Jedelsky!)
* Dropped official support for Python 3.2 (although it still works on 3.2)
* New `--speed` option for `asciinema play` (thanks Bastiaan "@bastiaanb" Bakker!)
* Ability to set API token via `ASCIINEMA_API_TOKEN` env variable (thanks Samantha "@samdmarshall" Marshall!)
* Improved shutdown on more signals: CHLD, HUP, TERM, QUIT (thanks Richard "@typerlc"!)
* Fixed stdin handling during playback via `asciinema play`

## 1.3.0 (2016-07-13)

This release brings back the original Python implementation of asciinema. It's
based on 0.9.8 codebase and adds all features and bug fixes that have been
implemented in asciinema's Go version between 0.9.8 and 1.2.0.

Other notable changes:

* Zero dependencies! (other than Python 3)
* Fixed crash when resizing terminal window during recording (#167)
* Fixed upload from IPv6 hosts (#94)
* Improved UTF-8 charset detection (#160)
* `-q/--quiet` option can be saved in config file now
* Final "logout" (produced by csh) is now removed from recorded stdout
* `rec` command now tries to write to target path before starting recording

## 1.2.0 (2016-02-22)

* Added playback from stdin: `cat demo.json | asciinema play -`
* Added playback from IPFS: `asciinema play ipfs:/ipfs/QmcdXYJp6e4zNuimuGeWPwNMHQdxuqWmKx7NhZofQ1nw2V`
* Added playback from asciicast page URL: `asciinema play https://asciinema.org/a/22124`
* `-q/--quiet` option added to `rec` command
* Fixed handling of partial UTF-8 sequences in recorded stdout
* Final "exit" is now removed from recorded stdout
* Longer operations like uploading/downloading show "spinner"

## 1.1.1 (2015-06-21)

* Fixed putting terminal in raw mode (fixes ctrl-o in nano)

## 1.1.0 (2015-05-25)

* `--max-wait` option is now also available for `play` command
* Added support for compilation on FreeBSD
* Improved locale/charset detection
* Improved upload error messages
* New config file location (with backwards compatibility)

## 1.0.0 (2015-03-12)

* `--max-wait` and `--yes` options can be saved in config file
* Support for displaying warning messages returned from API
* Also, see changes for 1.0.0 release candidates below

## 1.0.0.rc2 (2015-03-08)

* All dependencies are vendored now in Godeps dir
* Help message includes all commands with their possible options
* `-y` and `-t` options have longer alternatives: `--yes`, `--title`
* `--max-wait` option has shorter alternative: `-w`
* Import paths changed to `github.com/asciinema/asciinema` due to repository
  renaming
* `-y` also suppresess "please resize terminal" prompt

## 1.0.0.rc1 (2015-03-02)

* New [asciicast file format](doc/asciicast-v1.md)
* `rec` command can now record to file
* New commands: `play <filename>` and `upload <filename>`
* UTF-8 native locale is now required
* Added handling of status 413 and 422 by printing user friendly message

## 0.9.9 (2014-12-17)

* Rewritten in Go
* License changed to GPLv3
* `--max-wait` option added to `rec` command
* Recorded process has `ASCIINEMA_REC` env variable set (useful for "rec"
  indicator in shell's `$PROMPT/$RPROMPT`)
* No more terminal resetting (via `reset` command) before and after recording
* Informative messages are coloured to be distinguishable from normal output
* Improved error messages

## 0.9.8 (2014-02-09)

* Rename user_token to api_token
* Improvements to test suite
* Send User-Agent including client version number, python version and platform
* Handle 503 status as server maintenance
* Handle 404 response as a request for client upgrade

## 0.9.7 (2013-10-07)

* Depend on requests==1.1.0, not 2.0

## 0.9.6 (2013-10-06)

* Remove install script
* Introduce proper python package: https://pypi.python.org/pypi/asciinema
* Make the code compatible with both python 2 and 3
* Use requests lib instead of urrlib(2)

## 0.9.5 (2013-10-04)

* Fixed measurement of total recording time
* Improvements to install script
* Introduction of Homebrew formula

## 0.9.4 (2013-10-03)

* Use python2.7 in shebang

## 0.9.3 (2013-10-03)

* Re-enable resetting of a terminal before and after recording
* Add Arch Linux source package

## 0.9.2 (2013-10-02)

* Use os.uname over running the uname command
* Add basic integration tests
* Make PtyRecorder test stable again
* Move install script out of bin dir

## 0.9.1 (2013-10-01)

* Split monolithic script into separate classes/files
* Remove upload queue
* Use python2 in generated binary's shebang
* Delay config file creation until user_token is requested
* Introduce command classes for handling cli commands
* Split the recorder into classes with well defined responsibilities
* Drop curl dependency, use urllib(2) for http requests

## 0.9.0 (2013-09-24)

* Project rename from "ascii.io" to "asciinema"

## ... limbo? ...

## 0.1 (2012-03-11)

* Initial release
