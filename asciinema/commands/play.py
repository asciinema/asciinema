from asciinema.commands.command import Command
from asciinema.player import Player
import asciinema.asciicast as asciicast


class PlayCommand(Command):

    def __init__(self, filename, player=None):
        Command.__init__(self)
        self.filename = filename
        self.player = player if player is not None else Player()

    def execute(self):
        try:
            self.player.play(asciicast.load(self.filename))

        except FileNotFoundError as e:
            self.print_warning("Playback failed: %s" % str(e))
            return 1

        return 0
