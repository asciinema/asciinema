from asciinema.commands.command import Command
from asciinema.api import APIError


class UploadCommand(Command):
    """
    Uploads file cast to server

    Attributes:
        filename (str): name of file that will be uploaded
    """
    def __init__(self, api, filename):

        Command.__init__(self)
        self.api = api
        self.filename = filename

    def execute(self):
        """
        Attempts to upload file

        Returns:
            0 if successful, 1 otherwise
        """
        try:
            url, warn = self.api.upload_asciicast(self.filename)

            if warn:
                self.print_warning(warn)

            self.print(url)

        except OSError as e:
            self.print_error("upload failed: %s" % str(e))
            return 1

        except APIError as e:
            self.print_error("upload failed: %s" % str(e))
            self.print_error("retry later by running: asciinema upload %s" % self.filename)
            return 1

        return 0
