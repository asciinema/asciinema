from asciinema.commands.command import Command
from asciinema.player import Player
import asciinema.asciicast as asciicast


class PlayCommand(Command):

    def __init__(self, filename, idle_time_limit, speed, player=None):
        Command.__init__(self)
        self.filename = filename
        self.idle_time_limit = idle_time_limit
        self.speed = speed
        self.player = player if player is not None else Player()

    def execute(self):
        try:
            with asciicast.open_from_url(self.filename) as a:
                self.player.play(a, self.idle_time_limit, self.speed)

        except asciicast.LoadError as e:
            self.print_error("playback failed: %s" % str(e))
            return 1
        except KeyboardInterrupt:
            return 1

        return 0
