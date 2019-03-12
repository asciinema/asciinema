from asciinema.commands.command import Command
import asciinema.asciicast as asciicast


class EditCommand(Command):

    def __init__(self, source_files, target_file):
        self.source_files = source_files
        self.target_file = target_file

    def execute(self):
        try:
            for source in self.source_files:
                with asciicast.open_from_url(source) as s:
                    print(s) 

        except asciicast.LoadError as e:
            self.print_error("playback failed: %s" % str(e))
            return 1

        return 0