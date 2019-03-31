import sys

from asciinema.commands.command import Command
import asciinema.asciicast as asciicast


class CatCommand(Command):

    def __init__(self, args, config, env):
        Command.__init__(self, args, config, env)
        self.filename = args.filename

    def execute(self):
        try:
            with asciicast.open_from_url(self.filename) as a:
                for t, _type, text in a.stdout_events():
                    sys.stdout.write(text)
                    sys.stdout.flush()

        except asciicast.LoadError as e:
            self.print_error("printing failed: %s" % str(e))
            return 1

        return 0
