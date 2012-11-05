import json
import os
import shutil
import subprocess
import tempfile
import time

from timed_file import TimedFile


class Asciicast(object):

    def __init__(self):
        self.dir_path = tempfile.mkdtemp()

        self.command = None
        self.title = None
        self.shell = os.environ['SHELL']
        self.term = os.environ['TERM']
        self.username = os.environ['USER']
        self.uname = get_command_output(['uname', '-srvp'])
        self.recorded_at = time.time()

        self.stdout_file = TimedFile(self.dir_path + '/stdout')
        self.stdin_file  = TimedFile(self.dir_path + '/stdin')

    @property
    def metadata_filename(self):
        return self.dir_path + '/meta.json'

    @property
    def stdout_data_filename(self):
        return self.stdout_file.data_filename

    @property
    def stdout_timing_filename(self):
        return self.stdout_file.timing_filename

    @property
    def stdin_data_filename(self):
        return self.stdin_file.data_filename

    @property
    def stdin_timing_filename(self):
        return self.stdin_file.timing_filename

    # def open_files(self):
    #     self.stdout_file.start_timing()
    #     self.stdin_file.start_timing()

    def save(self):
        self.save_streams()
        self.save_metadata()

    def save_streams(self):
        self.stdout_file.close()
        self.stdin_file.close()

    def save_metadata(self):
        lines = int(get_command_output(['tput', 'lines']))
        columns = int(get_command_output(['tput', 'cols']))

        # RFC 2822
        recorded_at = time.strftime("%a, %d %b %Y %H:%M:%S +0000",
                                    time.gmtime(self.recorded_at))
        data = {
            'username'   : self.username,
            'user_token' : self.user_token,
            'duration'   : self.duration,
            'recorded_at': recorded_at,
            'title'      : self.title,
            'command'    : self.command,
            'shell'      : self.shell,
            'uname'      : self.uname,
            'term'       : {
                'type'   : self.term,
                'lines'  : lines,
                'columns': columns
            }
        }

        json_string = json.dumps(data, sort_keys=True, indent=4)
        with open(self.metadata_filename, 'wb') as f:
            f.write(json_string + '\n')

    def remove(self):
        shutil.rmtree(self.dir_path)


def get_command_output(args):
    process = subprocess.Popen(args, stdout=subprocess.PIPE)
    return process.communicate()[0].strip()
