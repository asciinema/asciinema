import sys
import os
import subprocess
import tempfile

import asciinema.util as util
from asciinema.recorder import Recorder
from asciinema.api import APIError


class RecordCommand:

    def __init__(self, api, filename, command, title, assume_yes, recorder=None):
        self.api = api
        self.filename = filename
        self.command = command
        self.title = title
        self.assume_yes = assume_yes
        self.recorder = recorder if recorder is not None else Recorder()

    def execute(self):
        if self.filename == "":
            self.filename = self._tmp_path()
            upload = True
        else:
            upload = False

        util.printf("Asciicast recording started.")
        util.printf("""Hit Ctrl-D or type "exit" to finish.""")

        self.recorder.record(self.filename, self.command, self.title)

        util.printf("Asciicast recording finished.")

        if upload:
            if not self.assume_yes:
                util.printf("Press <Enter> to upload, <Ctrl-C> to cancel.")
                try:
                    sys.stdin.readline()
                except KeyboardInterrupt:
                    return 0

            try:
                url, warn = self.api.upload_asciicast(self.filename)
                if warn:
                    util.warningf(warn)
                os.remove(self.filename)
                print(url)
            except APIError as e:
                util.warningf("Upload failed: {}".format(str(e)))
                util.warningf("Retry later by running: asciinema upload {}".format(self.filename))
                return 1

        return 0

    def _tmp_path(self):
        fd, path = tempfile.mkstemp(suffix='-asciinema.json')
        os.close(fd)
        return path
