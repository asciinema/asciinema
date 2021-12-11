import asciinema.asciicast as asciicast
from asciinema.commands.command import Command
from asciinema.player import Player


class PlayCommand(Command):
    def __init__(self, args, config, env, player=None):
        Command.__init__(self, args, config, env)
        self.filename = args.filename
        self.idle_time_limit = args.idle_time_limit
        self.speed = args.speed
        self.player = player if player is not None else Player()
        self.key_bindings = {
            "pause": config.play_pause_key,
            "step": config.play_step_key,
        }

    def execute(self):
        try:
            with asciicast.open_from_url(self.filename) as a:
                self.player.play(
                    a, self.idle_time_limit, self.speed, self.key_bindings
                )

        except asciicast.LoadError as e:
            self.print_error("playback failed: %s" % str(e))
            return 1
        except KeyboardInterrupt:
            return 1

        return 0
