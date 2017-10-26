import sys
import os
import tempfile

from asciinema.commands.command import Command
from asciinema.asciicast.v2 import Recorder
from asciinema.api import APIError


class RecordCommand(Command):

    def __init__(self, api, filename, rec_stdin, command, env_whitelist, title, assume_yes, quiet, idle_time_limit, recorder=None):
        Command.__init__(self, quiet)
        self.api = api
        self.filename = filename
        self.rec_stdin = rec_stdin
        self.command = command
        self.env_whitelist = env_whitelist
        self.title = title
        self.assume_yes = assume_yes or quiet
        self.idle_time_limit = idle_time_limit
        self.recorder = recorder if recorder is not None else Recorder()

    def execute(self):
        if self.filename == "":
            self.filename = _tmp_path()
            upload = True
        else:
            upload = False

        try:
            _touch(self.filename)
            os.remove(self.filename)
        except OSError as e:
            self.print_error("Can't write to %s: %s" % (self.filename, str(e)))
            return 1

        self.print_info("Recording asciicast to %s" % self.filename)
        self.print_info("""Hit <Ctrl-D> or type "exit" when you're done.""")

        self.recorder.record(
            self.filename,
            self.rec_stdin,
            self.command,
            self.env_whitelist,
            self.title,
            self.idle_time_limit
        )

        self.print_info("Recording finished.")

        if upload:
            if not self.assume_yes:
                self.print_info("Press <Enter> to upload to %s, <Ctrl-C> to save locally." % self.api.hostname())
                try:
                    sys.stdin.readline()
                except KeyboardInterrupt:
                    self.print("\r", end="")
                    self.print_info("Asciicast saved to %s" % self.filename)
                    return 0

            try:
                url, warn = self.api.upload_asciicast(self.filename)
                if warn:
                    self.print_warning(warn)
                os.remove(self.filename)
                self.print(url)
            except APIError as e:
                self.print("\r\x1b[A", end="")
                self.print_error("Upload failed: %s" % str(e))
                self.print_error("Retry later by running: asciinema upload %s" % self.filename)
                return 1
        else:
            self.print_info("Asciicast saved to %s" % self.filename)

        return 0


def _tmp_path():
    fd, path = tempfile.mkstemp(suffix='-ascii.cast')
    os.close(fd)
    return path


def _touch(path):
    open(path, 'a').close()
