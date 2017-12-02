import sys

from asciinema.commands.command import Command
import asciinema.asciicast as asciicast


class CatCommand(Command):

    def __init__(self, filename):
        Command.__init__(self)
        self.filename = filename

    def execute(self):
        try:
            with asciicast.open_from_url(self.filename) as a:
                for t, text in a.stdout():
                    sys.stdout.write(text)
                    sys.stdout.flush()

        except asciicast.LoadError as e:
            self.print_error("printing failed: %s" % str(e))
            return 1

        return 0
