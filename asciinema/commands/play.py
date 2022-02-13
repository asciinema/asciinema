from typing import Any, Dict, Optional

from .. import asciicast
from ..commands.command import Command
from ..config import Config
from ..player import Player


class PlayCommand(Command):
    def __init__(
        self,
        args: Any,
        config: Config,
        env: Dict[str, str],
        player: Optional[Player] = None,
    ) -> None:
        Command.__init__(self, args, config, env)
        self.filename = args.filename
        self.idle_time_limit = args.idle_time_limit
        self.speed = args.speed
        self.player = player if player is not None else Player()
        self.key_bindings = {
            "pause": config.play_pause_key,
            "step": config.play_step_key,
        }

    def execute(self) -> int:
        try:
            with asciicast.open_from_url(self.filename) as a:
                self.player.play(
                    a, self.idle_time_limit, self.speed, self.key_bindings
                )

        except asciicast.LoadError as e:
            self.print_error(f"playback failed: {str(e)}")
            return 1
        except KeyboardInterrupt:
            return 1

        return 0
