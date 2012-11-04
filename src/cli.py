import os
import sys

import asciicasts
from config import Config
from options import Options
from uploader import Uploader
import asciicast_recorder


SCRIPT_NAME = os.path.basename(sys.argv[0])

config = Config()
options = Options(sys.argv)


def run():
    action = options.action

    if action == 'rec':
        record()
    elif action == 'upload':
        upload_all_pending()
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
    # check_pending()

    # id = int(time.time())
    # path = "%s/%i" % (queue_dir_path, id)
    # asciicast = Asciicast(path)

    asciicast = Asciicast()
    asciicast.command = options.command
    asciicast.title = options.title

    start_time = time.time()

    if sys.stdin.isatty():
        record_process(command, asciicast.stdout_file)
    else:
        record_stdin(asciicast.stdout_file)

    end_time = time.time()

    asciicast.recorded_at = start_time
    asciicast.duration = end_time - start_time

    asciicast.save()

    # asciicast = asciicast_recorder.record(
    #         config.queue_dir_path, config.user_token, options
    #         )
    # asciicast = recorder.record()

    if is_upload_requested():
        print '~ Uploading...'
        upload_asciicast(asciicast)


def upload_all_pending():
    print 'Uploading pending asciicasts...'
    for asciicast in pending_asciicasts():
        upload_asciicast(asciicast)


def authenticate():
    url = '%s/connect/%s' % (config.api_url, config.user_token)
    print 'Open following URL in your browser to authenticate and/or ' \
        'claim recorded asciicasts:\n\n%s' % url


def print_help():
    print HELP_TEXT


def print_version():
    print 'asciiio 1.0.1'


def handle_unknown_action(action):
    print('Unknown action: %s' % action)
    print('Run "%s --help" for list of available options' % SCRIPT_NAME)
    sys.exit(1)


# Helpers

def check_pending():
    num = len(pending_asciicasts())
    if num > 0:
        print "Warning: %i recorded asciicasts weren't uploaded. " \
                'Run "%s upload" to upload them or delete them with ' \
                '"rm -rf %s/*".' \
                % (num, SCRIPT_NAME, config.queue_dir_path)


def pending_asciicasts():
    return asciicasts.pending(config.queue_dir_path)


def upload_asciicast(asciicast):
    uploader = Uploader(config.api_url)
    url = uploader.upload(asciicast)

    if url:
        print url
        asciicast.destroy()


def is_upload_requested():
    if options.always_yes:
        return True

    sys.stdout.write("~ Do you want to upload it? [Y/n] ")
    answer = sys.stdin.readline().strip()
    return answer == 'y' or answer == 'Y' or answer == ''


HELP_TEXT = '''usage: %s [-h] [-i] [-y] [-c <command>] [-t <title>] [action]

Asciicast recorder+uploader.

Actions:
 rec           record asciicast (this is the default when no action given)
 upload        upload recorded (but not uploaded) asciicasts
 auth          authenticate and/or claim recorded asciicasts

Optional arguments:
 -c command    run specified command instead of shell ($SHELL)
 -t title      specify title of recorded asciicast
 -y            don't prompt for confirmation
 -h, --help    show this help message and exit
 --version     show version information''' % SCRIPT_NAME
