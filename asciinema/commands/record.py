import sys
import os
import tempfile
import stat

from asciinema.commands.command import Command
import asciinema.asciicast as asciicast
from asciinema.asciicast.v2 import Recorder
from asciinema.api import APIError


class RecordCommand(Command):

    def __init__(self, api, args, recorder=None):
        Command.__init__(self, args.quiet)
        self.api = api
        self.filename = args.filename
        self.rec_stdin = args.stdin
        self.command = args.command
        self.env_whitelist = args.env
        self.title = args.title
        self.assume_yes = args.yes or args.quiet
        self.idle_time_limit = args.idle_time_limit
        self.append = args.append
        self.recorder = recorder if recorder is not None else Recorder()

    def execute(self):
        upload = False
        append = self.append
        start_time_offset = 0

        if self.filename == "":
            self.filename = _tmp_path()
            upload = True

        if os.path.exists(self.filename):
            if not os.access(self.filename, os.W_OK):
                self.print_error("Can't write to %s" % self.filename)
                return 1

            if os.stat(self.filename).st_size > 0:
                if append:
                    with asciicast.open_from_url(self.filename) as a:
                        for last_frame in a.stdout():
                            pass
                        start_time_offset = last_frame[0]
                else:
                    self.print_error("%s already exists, aborting." % self.filename)
                    self.print_error("Use --append option if you want to append to existing recording.")
                    return 1

        self.print_info("Recording asciicast to %s" % self.filename)
        self.print_info("""Hit <Ctrl-D> or type "exit" when you're done.""")

        self.recorder.record(
            self.filename,
            self.rec_stdin,
            self.command,
            self.env_whitelist,
            self.title,
            self.idle_time_limit,
            start_time_offset
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
