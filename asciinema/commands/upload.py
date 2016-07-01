from asciinema.commands.command import Command
from asciinema.api import APIError


class UploadCommand(Command):

    def __init__(self, api, filename):
        Command.__init__(self)
        self.api = api
        self.filename = filename

    def execute(self):
        try:
            url, warn = self.api.upload_asciicast(self.filename)

            if warn:
                self.print_warning(warn)

            self.print(url)

        except FileNotFoundError as e:
            self.print_warning("Upload failed: %s" % str(e))
            return 1

        except APIError as e:
            self.print_warning("Upload failed: %s" % str(e))
            self.print_warning("Retry later by running: asciinema upload %s" % self.filename)
            return 1

        return 0
