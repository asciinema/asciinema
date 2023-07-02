import sys
from typing import Any, Dict

from .. import asciicast
from ..config import Config
from ..tty_ import raw
from .command import Command


class CatCommand(Command):
    def __init__(self, args: Any, config: Config, env: Dict[str, str]):
        Command.__init__(self, args, config, env)
        self.filenames = args.filename

    def execute(self) -> int:
        try:
            with open("/dev/tty", "rt", encoding="utf-8") as stdin:
                with raw(stdin.fileno()):
                    return self.cat()
        except OSError:
            return self.cat()

    def cat(self) -> int:
        try:
            for filename in self.filenames:
                with asciicast.open_from_url(filename) as a:
                    for _, _type, text in a.events("o"):
                        sys.stdout.write(text)
                        sys.stdout.flush()

        except asciicast.LoadError as e:
            self.print_error(f"printing failed: {str(e)}")
            return 1

        return 0
