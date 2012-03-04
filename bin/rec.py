#!/usr/bin/env python

import sys
import os
import pty
import signal
import tty
import array
import termios
import fcntl
import select
import time
import json
import getopt
import subprocess
import httplib, urllib
import socket
import glob
import bz2
import ConfigParser
import uuid

SCRIPT_NAME = os.path.basename(sys.argv[0])


class AsciiCast(object):
    BASE_DIR = os.path.expanduser("~/.ascii.io")
    QUEUE_DIR = BASE_DIR + "/queue"

    def __init__(self, api_url, user_token, command, title, record_input):
        self.api_url = api_url
        self.user_token = user_token
        self.path = AsciiCast.QUEUE_DIR + "/%i" % int(time.time())
        self.command = command
        self.title = title
        self.record_input = record_input
        self.duration = None

    def create(self):
        self._record()
        return self._upload()

    def _record(self):
        os.makedirs(self.path)
        self.recording_start = time.time()
        command = self.command or os.environ['SHELL'].split()
        PtyRecorder(self.path, command, self.record_input).run()
        self.duration = time.time() - self.recording_start
        self._save_metadata()

    def _save_metadata(self):
        info_file = open(self.path + '/meta.json', 'wb')

        # RFC 2822
        recorded_at = time.strftime("%a, %d %b %Y %H:%M:%S +0000",
                                    time.gmtime(self.recording_start))

        command = self.command and ' '.join(self.command)
        uname = self._get_cmd_output(['uname', '-srvp'])
        shell = os.environ['SHELL']
        term = os.environ['TERM']
        lines = int(self._get_cmd_output(['tput', 'lines']))
        columns = int(self._get_cmd_output(['tput', 'cols']))

        data = {
            'user_token' : self.user_token,
            'duration'   : self.duration,
            'recorded_at': recorded_at,
            'title'      : self.title,
            'command'    : command,
            'shell'      : shell,
            'uname'      : uname,
            'term'       : {
                'type'   : term,
                'lines'  : lines,
                'columns': columns
            }
        }

        json_string = json.dumps(data, sort_keys=True, indent=4)
        info_file.write(json_string + '\n')
        info_file.close()

    def _get_cmd_output(self, args):
        process = subprocess.Popen(args, stdout=subprocess.PIPE)
        return process.communicate()[0].strip()

    def _upload(self):
        url = Uploader(self.api_url, self.path).upload()
        if url:
            print url
            return True
        else:
            return False


class TimedFile(object):
    '''File wrapper that records write times in separate file.'''

    def __init__(self, filename):
        mode = 'w'
        self.data_file = bz2.BZ2File(filename, mode)
        self.time_file = bz2.BZ2File(filename + '.time', mode)
        self.old_time = time.time()

    def write(self, data):
        self.data_file.write(data)
        now = time.time()
        delta = now - self.old_time
        self.time_file.write("%f %d\n" % (delta, len(data)))
        self.old_time = now

    def close(self):
        self.data_file.close()
        self.time_file.close()


class PtyRecorder(object):
    '''Pseudo-terminal recorder.

    Creates new pseudo-terminal for spawned process
    and saves stdin/stderr (and timing) to files.
    '''

    def __init__(self, path, command, record_input):
        self.master_fd = None
        self.path = path
        self.command = command
        self.record_input = record_input

    def run(self):
        self._open_files()
        self._write_stdout('\n~ Asciicast recording started.\n')
        success = self._spawn()
        self._write_stdout('\n~ Asciicast recording finished.\n')
        self._close_files()
        return success

    def _open_files(self):
        self.stdout_file = TimedFile(self.path + '/stdout')
        if self.record_input:
            self.stdin_file = TimedFile(self.path + '/stdin')

    def _close_files(self):
        self.stdout_file.close()
        if self.record_input:
            self.stdin_file.close()

    def _spawn(self):
        '''Create a spawned process.

        Based on pty.spawn() from standard library.
        '''

        assert self.master_fd is None

        pid, self.master_fd = pty.fork()

        if pid == pty.CHILD:
            os.execlp(self.command[0], *self.command)

        old_handler = signal.signal(signal.SIGWINCH, self._signal_winch)

        try:
            mode = tty.tcgetattr(pty.STDIN_FILENO)
            tty.setraw(pty.STDIN_FILENO)
            restore = 1
        except tty.error: # This is the same as termios.error
            restore = 0

        self._set_pty_size()

        try:
            self._copy()
        except (IOError, OSError):
            if restore:
                tty.tcsetattr(pty.STDIN_FILENO, tty.TCSAFLUSH, mode)

        os.close(self.master_fd)
        self.master_fd = None
        signal.signal(signal.SIGWINCH, old_handler)

        return True

    def _signal_winch(self, signal, frame):
        '''Signal handler for SIGWINCH - window size has changed.'''

        self._set_pty_size()

    def _set_pty_size(self):
        '''
        Sets the window size of the child pty based on the window size
        of our own controlling terminal.
        '''

        assert self.master_fd is not None

        # Get the terminal size of the real terminal, set it on the pseudoterminal.
        buf = array.array('h', [0, 0, 0, 0])
        fcntl.ioctl(pty.STDOUT_FILENO, termios.TIOCGWINSZ, buf, True)
        fcntl.ioctl(self.master_fd, termios.TIOCSWINSZ, buf)

    def _copy(self):
        '''Main select loop.

        Passes control to self._master_read() or self._stdin_read()
        when new data arrives.
        '''

        assert self.master_fd is not None

        while 1:
            try:
                rfds, wfds, xfds = select.select([self.master_fd, pty.STDIN_FILENO], [], [])
            except select.error, e:
                if e[0] == 4:   # Interrupted system call.
                    continue

            if self.master_fd in rfds:
                data = os.read(self.master_fd, 1024)

                if len(data) == 0:
                  break

                self._handle_master_read(data)

            if pty.STDIN_FILENO in rfds:
                data = os.read(pty.STDIN_FILENO, 1024)
                self._handle_stdin_read(data)

    def _handle_master_read(self, data):
        '''Handles new data on child process stdout.'''

        self._write_stdout(data)
        self.stdout_file.write(data)

    def _handle_stdin_read(self, data):
        '''Handles new data on child process stdin.'''

        self._write_master(data)
        if self.record_input:
            self.stdin_file.write(data)

    def _write_stdout(self, data):
        '''Writes to stdout as if the child process had written the data.'''

        os.write(pty.STDOUT_FILENO, data)

    def _write_master(self, data):
        '''Writes to the child process from its controlling terminal.'''

        assert self.master_fd is not None

        while data != '':
            n = os.write(self.master_fd, data)
            data = data[n:]


class Uploader(object):
    '''Asciicast uploader.

    Uploads recorded script to website using HTTP based API.
    '''

    def __init__(self, api_url, path):
        self.api_url = api_url
        self.path = path

    def upload(self):
        files = {
                     'meta': 'meta.json',
                   'stdout': 'stdout',
            'stdout_timing': 'stdout.time'
        }

        if os.path.exists(self.path + '/stdin'):
            files['stdin']        = 'stdin'
            files['stdin_timing'] = 'stdin.time'

        fields = ["-F asciicast[%s]=@%s/%s" % (f, self.path, files[f]) for f in files]

        cmd = "curl -sS -o - %s %s" % (' '.join(fields), '%s/asciicasts' % self.api_url)

        process = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE)
        stdout, stderr = process.communicate()

        if stderr:
            # print >> sys.stderr, stderr
            # sys.stderr.write(stderr)
            os.write(2, stderr)

        if stdout:
            return stdout
        else:
            return None


def check_pending():
    num = len(pending_list())
    if num > 0:
        print 'Warning: %i recorded asciicasts weren\'t uploaded. ' \
              'Run "%s -u" to upload them or delete them with "rm -rf %s/*".' \
              % (num, SCRIPT_NAME, AsciiCast.QUEUE_DIR)


def upload_pending():
    print 'Uploading pending asciicasts...'
    for path in pending_list():
        url = Uploader(path).upload()
        if url:
            print url


def pending_list():
    return [os.path.dirname(p) for p in glob.glob(AsciiCast.QUEUE_DIR + '/*/*.time')]


def usage():
    text = '''usage: %s [-h] [-i] [-c <command>] [-t <title>] [action]

Asciicast recorder+uploader.

Actions:
 rec           record asciicast (this is the default when no action given)
 upload        upload recorded (but not uploaded) asciicasts

Optional arguments:
 -i            record stdin (keystrokes will be shown during replay)
 -c command    run specified command instead of shell ($SHELL)
 -t title      specify title of recorded asciicast
 -h, --help    show this help message and exit
 --version     show version information''' % SCRIPT_NAME
    print text


def print_version():
    print 'ascii.io-clio v0.x'


def main():
    '''Parses command-line options and creates asciicast.'''

    try:
        opts, args = getopt.getopt(sys.argv[1:], 'c:t:ih', ['help', 'version'])
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
        api_url = 'http://ascii.io/api'

    with open(cfg_file, 'wb') as configfile:
        config.write(configfile)

    api_url = os.environ.get('ASCII_IO_API_URL', api_url)

    command = None
    title = None

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

    if action == 'rec':
        check_pending()
        if not AsciiCast(api_url, user_token, command, title, record_input).create():
            sys.exit(1)
    elif action == 'upload':
        upload_pending()
    else:
        print('Unknown action: %s' % action)
        print('Run "%s --help" for list of available options' % sys.argv[0])


if __name__ == '__main__':
    main()
