import sys


class ErrorCommand(object):

    def __init__(self, message):
        self.message = message

    def execute(self):
        print("asciinema: %s. See 'asciinema --help'." % self.message)
        sys.exit(1)
