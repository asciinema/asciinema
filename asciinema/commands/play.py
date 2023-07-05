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
        self.loop = args.loop
        self.out_fmt = args.out_fmt
        self.stream = args.stream
        self.pause_on_markers = args.pause_on_markers
        self.player = player if player is not None else Player()
        self.key_bindings = {
            "pause": config.play_pause_key,
            "step": config.play_step_key,
            "next_marker": config.play_next_marker_key,
        }

    def execute(self) -> int:
        code = self.play()

        if self.loop:
            while code == 0:
                code = self.play()

        return code

    def play(self) -> int:
        try:
            with asciicast.open_from_url(self.filename) as a:
                self.player.play(
                    a,
                    idle_time_limit=self.idle_time_limit,
                    speed=self.speed,
                    key_bindings=self.key_bindings,
                    out_fmt=self.out_fmt,
                    stream=self.stream,
                    pause_on_markers=self.pause_on_markers,
                )

        except asciicast.LoadError as e:
            self.print_error(f"playback failed: {str(e)}")
            return 1
        except KeyboardInterrupt:
            return 1

        return 0
