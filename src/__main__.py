#!/usr/bin/env python

import os
import sys
import getopt
import httplib, urllib
import socket
import glob
import ConfigParser
import uuid

from constants import BASE_DIR, SCRIPT_NAME
from asciicast import AsciiCast
from uploader import Uploader

def check_pending():
    num = len(pending_list())
    if num > 0:
        print 'Warning: %i recorded asciicasts weren\'t uploaded. ' \
              'Run "%s upload" to upload them or delete them with "rm -rf %s/*".' \
              % (num, SCRIPT_NAME, AsciiCast.QUEUE_DIR)


def upload_pending(api_url):
    print 'Uploading pending asciicasts...'
    for path in pending_list():
        url = Uploader(api_url, path).upload()
        if url:
            print url


def auth(api_url, user_token):
    url = '%s/connect/%s' % (api_url, user_token)
    print 'Open following URL in your browser to authenticate and/or claim ' \
          'recorded asciicasts:\n\n%s' % url


def pending_list():
    return [os.path.dirname(p) for p in glob.glob(AsciiCast.QUEUE_DIR + '/*/*.time')]


def usage():
    text = '''usage: %s [-h] [-i] [-y] [-c <command>] [-t <title>] [action]

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
    print text


def print_version():
    print 'asciiio 1.0'


def main():
    '''Parses command-line options and creates asciicast.'''

    try:
        opts, args = getopt.getopt(sys.argv[1:], 'c:t:ihy', ['help', 'version'])
    except getopt.error as msg:
        print('%s: %s' % (sys.argv[0], msg))
        print('Run "%s --help" for list of available options' % sys.argv[0])
        sys.exit(2)

    action = 'rec'

    if len(args) > 1:
        print('Too many arguments')
        print('Run "%s --help" for list of available options' % sys.argv[0])
        sys.exit(2)
    elif len(args) == 1:
        action = args[0]

    config = ConfigParser.RawConfigParser()
    config.add_section('user')
    config.add_section('api')
    config.add_section('record')

    cfg_file = os.path.expanduser('~/.ascii.io/config')
    try:
        config.read(cfg_file)
    except ConfigParser.ParsingError:
        print('Config file %s contains syntax errors' % cfg_file)
        sys.exit(2)

    try:
        user_token = config.get('user', 'token')
    except ConfigParser.NoOptionError:
        user_token = str(uuid.uuid1())
        config.set('user', 'token', user_token)

    try:
        record_input = config.getboolean('record', 'input')
    except ConfigParser.NoOptionError:
        record_input = False

    try:
        api_url = config.get('api', 'url')
    except ConfigParser.NoOptionError:
        api_url = 'http://ascii.io'

    if not os.path.isdir(BASE_DIR):
        os.mkdir(BASE_DIR)

    if not os.path.exists(cfg_file):
        with open(cfg_file, 'wb') as configfile:
            config.write(configfile)

    api_url = os.environ.get('ASCII_IO_API_URL', api_url)

    command = None
    title = None
    always_yes = False

    for opt, arg in opts:
        if opt in ('-h', '--help'):
            usage()
            sys.exit(0)
        elif opt == '--version':
            print_version()
            sys.exit(0)
        elif opt == '-c':
            command = arg.split()
        elif opt == '-t':
            title = arg
        elif opt == '-i':
            record_input = True
        elif opt == '-y':
            always_yes = True

    if action == 'rec':
        check_pending()
        if not AsciiCast(api_url, user_token, command, title, record_input, always_yes).create():
            sys.exit(1)
    elif action == 'upload':
        upload_pending(api_url)
    elif action == 'auth':
        auth(api_url, user_token)
    else:
        print('Unknown action: %s' % action)
        print('Run "%s --help" for list of available options' % sys.argv[0])


if __name__ == '__main__':
    main()
