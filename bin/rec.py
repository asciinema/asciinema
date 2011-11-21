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

class TimedFile(object):
    '''File wrapper that records write times in separate file.'''

    def __init__(self, filename):
        mode = 'wb'
        self.data_file = open(filename, mode)
        self.time_file = open(filename + '.time', mode)
        self.old_time = time.time()

    def close(self):
        self.data_file.close()
        self.time_file.close()

    def write(self, data):
        self.data_file.write(data)
        now = time.time()
        delta = now - self.old_time
        self.time_file.write("%f %d\n" % (delta, len(data)))
        self.old_time = now


class PtyRecorder(object):
    '''Pseudo-terminal recorder.

    Creates new pseudo-terminal for spawned process
    and saves stdin/stderr (and timing) to files.
    '''

    def __init__(self, base_filename, command, record_input):
        self.master_fd = None
        self.base_filename = base_filename
        self.command = command
        self.record_input = record_input

    def run(self):
        self.open_files()
        self.write_stdout('\n~ Asciicast recording started.\n')
        success = self.spawn()
        self.write_stdout('\n~ Asciicast recording finished.\n')
        self.close_files()
        return success

    def open_files(self):
        self.stdout_file = TimedFile(self.base_filename + '.stdout')
        if self.record_input:
            self.stdin_file = TimedFile(self.base_filename + '.stdin')

    def close_files(self):
        self.stdout_file.close()
        if self.record_input:
            self.stdin_file.close()

    def spawn(self):
        '''Create a spawned process.

        Based on pty.spawn() from standard library.
        '''

        assert self.master_fd is None

        pid, master_fd = pty.fork()
        self.master_fd = master_fd

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

        os.close(master_fd)
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

        Passes control to self.master_read() or self.stdin_read()
        when new data arrives.
        '''

        assert self.master_fd is not None
        master_fd = self.master_fd

        while 1:
            try:
                rfds, wfds, xfds = select.select([master_fd, pty.STDIN_FILENO], [], [])
            except select.error, e:
                if e[0] == 4:   # Interrupted system call.
                    continue

            if master_fd in rfds:
                data = os.read(self.master_fd, 1024)
                self.handle_master_read(data)

            if pty.STDIN_FILENO in rfds:
                data = os.read(pty.STDIN_FILENO, 1024)
                self.handle_stdin_read(data)

    def handle_master_read(self, data):
        '''Handles new data on child process stdout.'''

        self.write_stdout(data)
        self.stdout_file.write(data)

    def handle_stdin_read(self, data):
        '''Handles new data on child process stdin.'''

        self.write_master(data)
        if self.record_input:
            self.stdin_file.write(data)

    def write_stdout(self, data):
        '''Writes to stdout as if the child process had written the data.'''

        os.write(pty.STDOUT_FILENO, data)

    def write_master(self, data):
        '''Writes to the child process from its controlling terminal.'''

        master_fd = self.master_fd
        assert master_fd is not None
        while data != '':
            n = os.write(master_fd, data)
            data = data[n:]


class AsciiCast(object):
    '''Asciicast model.

    Manages recording and uploading of asciicast.
    '''

    def __init__(self, command, title=None, record_input=False):
        self.base_filename = str(int(time.time()))
        self.command = command
        self.title = title
        self.record_input = record_input

    def create(self):
        ret = self.record()
        if ret:
            self.write_metadata()
            self.upload()

    def record(self):
        rec = PtyRecorder(self.base_filename, self.command, self.record_input)
        return rec.run()

    def write_metadata(self):
        info_file = open(self.base_filename + '.json', 'wb')

        json_data = {
                'title': self.title,
                'command': ' '.join(self.command),
                'term': {
                    'type': os.environ['TERM'],
                    'lines': int(self.get_output(['tput', 'lines'])),
                    'columns': int(self.get_output(['tput', 'cols'])),
                    },
                'shell': os.environ['SHELL'],
                'uname': self.get_output(['uname', '-osrvp'])
                }

        json_string = json.dumps(json_data, sort_keys=True, indent=2)
        info_file.write(json_string + '\n')
        info_file.close()

    def get_output(self, args):
        process = subprocess.Popen(args, stdout=subprocess.PIPE)
        return process.communicate()[0].strip()

    def upload(self):
        up = Uploader(self.base_filename)
        up.upload()


class Uploader(object):
    '''Asciicast uploader.

    Uploads recorded script to website using HTTP based API.
    '''

    def __init__(self, base_filename):
        self.api_host = os.environ.get('TTV_API_HOST', 'localhost:3000')
        self.api_path = '/scripts'
        self.base_filename = base_filename

    def upload(self):
        params = self.build_params()
        self.make_request(params)

    def make_request(self, params):
        headers = {"Content-type": "application/x-www-form-urlencoded", "Accept": "text/plain"}
        conn = httplib.HTTPConnection(self.api_host)
        try:
            conn.request("POST", self.api_path, params, headers)
        except socket.error:
            print "Oops, couldn't connect to ..."
            return

        response = conn.getresponse()

        if response.status == 201:
            print response.read()
        else:
            print 'Oops, something is not right. (%d: %s)' % (response.status,
                    response.read())

    def build_params(self):
        params = urllib.urlencode({
            'metadata': 'lolza'
            })

        return params


def main():
    '''Parses command-line options and creates asciicast.'''

    try:
        opts, args = getopt.getopt(sys.argv[1:], 'c:t:ih', ['help'])
    except getopt.error as msg:
        print('%s: %s' % (sys.argv[0], msg))
        print('Run "%s --help" for list of available options' % sys.argv[0])
        sys.exit(2)

    command = os.environ['SHELL'].split()
    title = None
    record_input = False

    for opt, arg in opts:
        if opt in ('-h', '--help'):
            usage()
            sys.exit(0)
        elif opt == '-c':
            command = arg.split()
        elif opt == '-t':
            title = arg
        elif opt == '-i':
            record_input = True

    ac = AsciiCast(command, title, record_input)
    ac.create()


def usage():
    text = '''usage: %s [-h] [-i] [-c <command>] [-t <title>]

Asciicast recorder+uploader.

optional arguments:
 -h, --help    show this help message and exit
 -i            record stdin (keystrokes will be shown during replay)
 -c command    run specified command instead of shell ($SHELL)
 -t title      specify title of recorded asciicast''' % sys.argv[0]
    print text

if __name__ == '__main__':
    main()
