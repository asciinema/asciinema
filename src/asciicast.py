import os
import sys
import time
import subprocess
import json
import shutil

from constants import SCRIPT_NAME, BASE_DIR
from pty_recorder import PtyRecorder
from uploader import Uploader

class AsciiCast(object):
    QUEUE_DIR = BASE_DIR + "/queue"

    def __init__(self, api_url, user_token, command, title, record_input, always_yes):
        self.api_url = api_url
        self.user_token = user_token
        self.path = AsciiCast.QUEUE_DIR + "/%i" % int(time.time())
        self.command = command
        self.title = title
        self.record_input = record_input
        self.duration = None
        self.always_yes = always_yes

    def create(self):
        self._record()
        if self.confirm_upload():
            return self._upload()
        else:
            self._delete()

    def confirm_upload(self):
        if self.always_yes:
            return True

        sys.stdout.write("~ Do you want to upload it? [Y/n] ")
        answer = sys.stdin.readline().strip()
        return answer == 'y' or answer == 'Y' or answer == ''

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
        username = os.environ['USER']
        shell = os.environ['SHELL']
        term = os.environ['TERM']
        lines = int(self._get_cmd_output(['tput', 'lines']))
        columns = int(self._get_cmd_output(['tput', 'cols']))

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

    def _delete(self):
        shutil.rmtree(self.path)
