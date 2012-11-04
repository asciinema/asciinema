import os
import time
import subprocess

from asciicasts import Asciicast
from recorders import ProcessRecorder


# def record(command, queue_dir_path):
#     # id = int(time.time())
#     # path = "%s/%i" % (queue_dir_path, id)
#     # asciicast = Asciicast(path)

#     if sys.stdin.isatty():
#         record_process(command, asciicast.stdout_file)
#     else:
#         record_stdin(asciicast.stdout_file)


# def record_stdin(stdout_file, stdin=sys.stdin):


# def record_process(command, stdout_file, stdin_file=None):


def record(queue_dir_path, user_token, options):
    # id = int(time.time())
    # path = "%s/%i" % (queue_dir_path, id)
    # asciicast = Asciicast(path)

    self.command = options.command
    self.record_input = options.record_input
    self.always_yes = options.always_yes

    self.recording_start_time = None
    self.duration = None

    self.record_command()
    self.save_metadata(user_token=user_token, title=options.title)

    return asciicast

def record_command(self):
    self.recording_start_time = time.time()
    cmd = self.command or os.environ['SHELL'].split()

    stdin_file = None
    if self.record_input:
        stdin_file = self.asciicast.stdin_file

    self.asciicast.open_files()

    recorder = ProcessRecorder(cmd, self.asciicast.stdout_file, stdin_file)
    recorder.run()

    now = time.time()
    self.duration = now - self.recording_start_time

    self.asciicast.close_files()

def save_metadata(self):
    # RFC 2822
    recorded_at = time.strftime("%a, %d %b %Y %H:%M:%S +0000",
                                time.gmtime(self.recording_start_time))

    command = self.command and ' '.join(self.command)
    uname = get_command_output(['uname', '-srvp'])
    username = os.environ['USER']
    shell = os.environ['SHELL']
    term = os.environ['TERM']
    lines = int(get_command_output(['tput', 'lines']))
    columns = int(get_command_output(['tput', 'cols']))

    data = {
        'username'   : username,
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

    self.asciicast.save_metadata(data)


def get_command_output(args):
    process = subprocess.Popen(args, stdout=subprocess.PIPE)
    return process.communicate()[0].strip()
