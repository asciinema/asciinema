import os
import sys

from asciicast import Asciicast
from config import Config
from options import Options
from uploader import Uploader
import recorders


SCRIPT_NAME = os.path.basename(sys.argv[0])

config = Config()
options = Options(sys.argv)


def run():
    action = options.action

    if action == 'rec':
        record()
    elif action == 'auth':
        authenticate()
    elif action == 'help':
        print_help()
    elif action == 'version':
        print_version()
    else:
        handle_unknown_action(action)


# Actions

def record():
    asciicast = record_asciicast()

    if upload_requested():
        print '~ Uploading...'
        upload_asciicast(asciicast)

    asciicast.remove()


def authenticate():
    url = '%s/connect/%s' % (config.api_url, config.user_token)
    print 'Open following URL in your browser to authenticate and/or ' \
        'claim recorded asciicasts:\n\n%s' % url


def print_help():
    print HELP_TEXT


def print_version():
    print 'asciinema 0.9.1'


def handle_unknown_action(action):
    print('Unknown action: %s' % action)
    print('Run "%s --help" for list of available options' % SCRIPT_NAME)
    sys.exit(1)


# Helpers

def record_asciicast():
    asciicast = Asciicast()

    if sys.stdin.isatty():
        if options.command:
            command = options.command
            is_shell = False
        else:
            command = os.environ['SHELL']
            is_shell = True

        stdin_file = asciicast.stdin_file if options.record_input else None
        duration = recorders.record_process(command, is_shell,
                                            asciicast.stdout_file, stdin_file)
    else:
        duration = recorders.record_stream(sys.stdin, asciicast.stdout_file)

    asciicast.user_token = config.user_token
    asciicast.command = options.command
    asciicast.title = options.title
    asciicast.duration = duration

    asciicast.save()

    return asciicast


def upload_asciicast(asciicast):
    uploader = Uploader(config.api_url)
    url = uploader.upload(asciicast)

    if url:
        print url


def upload_requested():
    if options.always_yes:
        return True

    sys.stdout.write("~ Do you want to upload it? [Y/n] ")
    answer = sys.stdin.readline().strip()
    return answer == 'y' or answer == 'Y' or answer == ''


HELP_TEXT = '''usage: %s [-h] [-i] [-y] [-c <command>] [-t <title>] [action]

Asciicast recorder+uploader.

Actions:
 rec           record asciicast (this is the default when no action given)
 auth          authenticate and/or claim recorded asciicasts

Optional arguments:
 -c command    run specified command instead of shell ($SHELL)
 -t title      specify title of recorded asciicast
 -y            don't prompt for confirmation
 -h, --help    show this help message and exit
 --version     show version information''' % SCRIPT_NAME
