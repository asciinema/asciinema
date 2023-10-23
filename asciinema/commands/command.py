import os
import sys
from typing import Any, Dict, Optional

from ..api import Api
from ..config import Config


class Command:
    def __init__(self, _args: Any, config: Config, env: Dict[str, str]):
        self.quiet: bool = False
        self.api = Api(config.api_url, env.get("USER"), config.install_id)

    def print(
        self,
        text: str,
        end: str = "\r\n",
        color: Optional[int] = None,
        force: bool = False,
        flush: bool = False,
    ) -> None:
        if not self.quiet or force:
            if color is not None and os.isatty(sys.stderr.fileno()):
                text = f"\x1b[0;3{color}m{text}\x1b[0m"

            print(text, file=sys.stderr, end=end)

            if flush:
                sys.stderr.flush()

    def print_info(self, text: str) -> None:
        self.print(f"asciinema: {text}", color=2)

    def print_warning(self, text: str) -> None:
        self.print(f"asciinema: {text}", color=3)

    def print_error(self, text: str) -> None:
        self.print(f"asciinema: {text}", color=1, force=True)
