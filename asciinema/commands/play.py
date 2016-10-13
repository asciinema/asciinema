from asciinema.commands.command import Command
from asciinema.player import Player
import asciinema.asciicast as asciicast


class PlayCommand(Command):

    def __init__(self, filename, max_wait, player=None, start_at=0):
        Command.__init__(self)
        self.filename = filename
        self.max_wait = max_wait
        self.player = player if player is not None else Player()
        self.start_at = start_at

    def execute(self):
        try:
            self.player.play(asciicast.load(self.filename), self.max_wait, self.start_at)

        except asciicast.LoadError as e:
            self.print_warning("Playback failed: %s" % str(e))
            return 1
        except KeyboardInterrupt:
            return 1

        return 0
