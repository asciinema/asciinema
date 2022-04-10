import os
import sys
from typing import Any, Dict

from ..api import Api
from ..config import Config


class Command:
    def __init__(self, _args: Any, config: Config, env: Dict[str, str]):
        self.quiet: bool = False
        self.api = Api(config.api_url, env.get("USER"), config.install_id)

    def print(
        self,
        text: str,
        end: str = "\n",
        force: bool = False,
    ) -> None:
        if not self.quiet or force:
            print(text, file=sys.stderr, end=end)

    def print_info(self, text: str) -> None:
        if os.isatty(sys.stderr.fileno()):
            self.print(f"\x1b[0;32masciinema: {text}\x1b[0m")
        else:
            self.print(f"asciinema: {text}")

    def print_warning(self, text: str) -> None:
        if os.isatty(sys.stderr.fileno()):
            self.print(f"\x1b[0;33masciinema: {text}\x1b[0m")
        else:
            self.print(f"asciinema: {text}")

    def print_error(self, text: str) -> None:
        if os.isatty(sys.stderr.fileno()):
            self.print(
                f"\x1b[0;31masciinema: {text}\x1b[0m",
                force=True,
            )
        else:
            self.print(f"asciinema: {text}", force=True)
