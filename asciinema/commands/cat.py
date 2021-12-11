import sys
from typing import Any, Dict

from .. import asciicast
from ..config import Config
from ..term import raw
from .command import Command


class CatCommand(Command):
    def __init__(self, args: Any, config: Config, env: Dict[str, str]):
        Command.__init__(self, args, config, env)
        self.filename = args.filename

    def execute(self) -> int:
        try:
            with open("/dev/tty", "wt", encoding="utf-8") as stdin:
                with raw(stdin.fileno()):
                    with asciicast.open_from_url(self.filename) as a:
                        for _, _type, text in a.stdout_events():
                            sys.stdout.write(text)
                            sys.stdout.flush()

        except asciicast.LoadError as e:
            self.print_error(f"printing failed: {str(e)}")
            return 1

        return 0
