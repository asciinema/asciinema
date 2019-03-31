import locale
import argparse
import os
import sys

from asciinema import __version__
import asciinema.config as config
from asciinema.commands.auth import AuthCommand
from asciinema.commands.record import RecordCommand
from asciinema.commands.play import PlayCommand
from asciinema.commands.cat import CatCommand
from asciinema.commands.upload import UploadCommand


def positive_float(value):
    value = float(value)
    if value <= 0.0:
        raise argparse.ArgumentTypeError("must be positive")

    return value


def maybe_str(v):
    if v is not None:
        return str(v)


def main():
    if locale.nl_langinfo(locale.CODESET).upper() not in ['US-ASCII', 'UTF-8']:
        print("asciinema needs an ASCII or UTF-8 character encoding to run. Check the output of `locale` command.")
        sys.exit(1)

    try:
        cfg = config.load()
    except config.ConfigError as e:
        sys.stderr.write(str(e) + '\n')
        sys.exit(1)

    # create the top-level parser
    parser = argparse.ArgumentParser(
        description="Record and share your terminal sessions, the right way.",
        epilog="""example usage:
  Record terminal and upload it to asciinema.org:
    \x1b[1masciinema rec\x1b[0m
  Record terminal to local file:
    \x1b[1masciinema rec demo.cast\x1b[0m
  Record terminal and upload it to asciinema.org, specifying title:
    \x1b[1masciinema rec -t "My git tutorial"\x1b[0m
  Record terminal to local file, limiting idle time to max 2.5 sec:
    \x1b[1masciinema rec -i 2.5 demo.cast\x1b[0m
  Replay terminal recording from local file:
    \x1b[1masciinema play demo.cast\x1b[0m
  Replay terminal recording hosted on asciinema.org:
    \x1b[1masciinema play https://asciinema.org/a/difqlgx86ym6emrmd8u62yqu8\x1b[0m
  Print full output of recorded session:
    \x1b[1masciinema cat demo.cast\x1b[0m

For help on a specific command run:
  \x1b[1masciinema <command> -h\x1b[0m""",
        formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument('--version', action='version', version='asciinema %s' % __version__)

    subparsers = parser.add_subparsers()

    # create the parser for the "rec" command
    parser_rec = subparsers.add_parser('rec', help='Record terminal session')
    parser_rec.add_argument('--stdin', help='enable stdin recording, disabled by default', action='store_true', default=cfg.record_stdin)
    parser_rec.add_argument('--append', help='append to existing recording', action='store_true', default=False)
    parser_rec.add_argument('--raw', help='save only raw stdout output', action='store_true', default=False)
    parser_rec.add_argument('--overwrite', help='overwrite the file if it already exists', action='store_true', default=False)
    parser_rec.add_argument('-c', '--command', help='command to record, defaults to $SHELL', default=cfg.record_command)
    parser_rec.add_argument('-e', '--env', help='list of environment variables to capture, defaults to ' + config.DEFAULT_RECORD_ENV, default=cfg.record_env)
    parser_rec.add_argument('-t', '--title', help='title of the asciicast')
    parser_rec.add_argument('-i', '--idle-time-limit', help='limit recorded idle time to given number of seconds', type=positive_float, default=maybe_str(cfg.record_idle_time_limit))
    parser_rec.add_argument('-y', '--yes', help='answer "yes" to all prompts (e.g. upload confirmation)', action='store_true', default=cfg.record_yes)
    parser_rec.add_argument('-q', '--quiet', help='be quiet, suppress all notices/warnings (implies -y)', action='store_true', default=cfg.record_quiet)
    parser_rec.add_argument('filename', nargs='?', default='', help='filename/path to save the recording to')
    parser_rec.set_defaults(cmd=RecordCommand)

    # create the parser for the "play" command
    parser_play = subparsers.add_parser('play', help='Replay terminal session')
    parser_play.add_argument('-i', '--idle-time-limit', help='limit idle time during playback to given number of seconds', type=positive_float, default=maybe_str(cfg.play_idle_time_limit))
    parser_play.add_argument('-s', '--speed', help='playback speedup (can be fractional)', type=positive_float, default=cfg.play_speed)
    parser_play.add_argument('filename', help='local path, http/ipfs URL or "-" (read from stdin)')
    parser_play.set_defaults(cmd=PlayCommand)

    # create the parser for the "cat" command
    parser_cat = subparsers.add_parser('cat', help='Print full output of terminal session')
    parser_cat.add_argument('filename', help='local path, http/ipfs URL or "-" (read from stdin)')
    parser_cat.set_defaults(cmd=CatCommand)

    # create the parser for the "upload" command
    parser_upload = subparsers.add_parser('upload', help='Upload locally saved terminal session to asciinema.org')
    parser_upload.add_argument('filename', help='filename or path of local recording')
    parser_upload.set_defaults(cmd=UploadCommand)

    # create the parser for the "auth" command
    parser_auth = subparsers.add_parser('auth', help='Manage recordings on asciinema.org account')
    parser_auth.set_defaults(cmd=AuthCommand)

    # parse the args and call whatever function was selected
    args = parser.parse_args()

    if hasattr(args, 'cmd'):
        command = args.cmd(args, cfg, os.environ)
        code = command.execute()
        sys.exit(code)
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == '__main__':
    main()
