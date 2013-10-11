import os
import subprocess
import time


class Asciicast(object):

    def __init__(self, env=os.environ):
        self.command = None
        self.title = None
        self.shell = env['SHELL']
        self.term = env['TERM']
        self.username = env['USER']

    @property
    def meta_data(self):
        lines = int(get_command_output(['tput', 'lines']))
        columns = int(get_command_output(['tput', 'cols']))

        return {
            'username'   : self.username,
            'duration'   : self.duration,
            'title'      : self.title,
            'command'    : self.command,
            'shell'      : self.shell,
            'term'       : {
                'type'   : self.term,
                'lines'  : lines,
                'columns': columns
            }
        }


def get_command_output(args):
    process = subprocess.Popen(args, stdout=subprocess.PIPE)
    return process.communicate()[0].strip()
