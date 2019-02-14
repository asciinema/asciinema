import sys

from asciinema.commands.command import Command
import asciinema.asciicast as asciicast


class CatCommand(Command):
    """
    Prints output of recorded asciicast session

    Attributes:
        filename (str): name of file that will be printed
    """

    def __init__(self, filename):

        Command.__init__(self)
        self.filename = filename

    def execute(self):
        """
        Attempts to open file and write contents to standard out

        Returns:
            0 if successful, 1 otherwise.
        """

        try:
            with asciicast.open_from_url(self.filename) as a:
                for t, _type, text in a.stdout_events():
                    sys.stdout.write(text)
                    sys.stdout.flush()

        except asciicast.LoadError as e:
            self.print_error("printing failed: %s" % str(e))
            return 1

        return 0
