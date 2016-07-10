import sys
import os
import tempfile

from asciinema.commands.command import Command
from asciinema.recorder import Recorder
from asciinema.api import APIError


class RecordCommand(Command):

    def __init__(self, api, filename, command, title, assume_yes, quiet, max_wait, recorder=None):
        Command.__init__(self, quiet)
        self.api = api
        self.filename = filename
        self.command = command
        self.title = title
        self.assume_yes = assume_yes or quiet
        self.max_wait = max_wait
        self.recorder = recorder if recorder is not None else Recorder()

    def execute(self):
        if self.filename == "":
            self.filename = _tmp_path()
            upload = True
        else:
            upload = False

        try:
            _touch(self.filename)
        except OSError as e:
            self.print_warning("Can't record to %s: %s" % (self.filename, str(e)))
            return 1

        self.print_info("Asciicast recording started.")
        self.print_info("""Hit Ctrl-D or type "exit" to finish.""")

        self.recorder.record(self.filename, self.command, self.title, self.max_wait)

        self.print_info("Asciicast recording finished.")

        if upload:
            if not self.assume_yes:
                self.print_info("Press <Enter> to upload, <Ctrl-C> to cancel.")
                try:
                    sys.stdin.readline()
                except KeyboardInterrupt:
                    return 0

            try:
                url, warn = self.api.upload_asciicast(self.filename)
                if warn:
                    self.print_warning(warn)
                os.remove(self.filename)
                self.print(url)
            except APIError as e:
                self.print_warning("Upload failed: %s" % str(e))
                self.print_warning("Retry later by running: asciinema upload %s" % self.filename)
                return 1

        return 0


def _tmp_path():
    fd, path = tempfile.mkstemp(suffix='-asciinema.json')
    os.close(fd)
    return path


def _touch(path):
    open(path, 'a').close()
