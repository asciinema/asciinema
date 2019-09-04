from asciinema.commands.command import Command
from asciinema.player import Player
import asciinema.asciicast as asciicast
import asciinema.term as term


class PlayCommand(Command):

    def __init__(self, args, config, env, player=None):
        Command.__init__(self, args, config, env)
        self.filename = args.filename
        self.idle_time_limit = args.idle_time_limit
        self.speed = args.speed
        self.player = player if player is not None else Player()

    def execute(self):
        try:
            with asciicast.open_from_url(self.filename) as a:
                play_w, play_h = term.get_size()
                rec_w, rec_h = a.get_size()
                if play_w < rec_w or play_h < rec_h:
                    self.print_warning("Terminal size is smaller than recording")
                    self.print_warning("Playback may not be displayed as intended")
                    self.print_warning('Trying making terminal at least {} x {} in size'.format(rec_w, rec_h))

                self.player.play(a, self.idle_time_limit, self.speed)

        except asciicast.LoadError as e:
            self.print_error("playback failed: %s" % str(e))
            return 1
        except KeyboardInterrupt:
            return 1

        return 0
