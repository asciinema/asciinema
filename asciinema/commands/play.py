from asciinema.commands.command import Command
from asciinema.player import Player
import asciinema.asciicast as asciicast


class PlayCommand(Command):

    def __init__(self, filename, max_wait, speed, player=None):
        Command.__init__(self)
        self.filename = filename
        self.max_wait = max_wait
        self.speed = speed
        self.player = player if player is not None else Player()

    def execute(self):
        try:
            self.player.play(asciicast.load(self.filename), self.max_wait, self.speed)

        except asciicast.LoadError as e:
            self.print_warning("Playback failed: %s" % str(e))
            return 1
        except KeyboardInterrupt:
            return 1

        return 0
