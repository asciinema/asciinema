import sys
from typing import Any, Dict, TextIO

from ..api import Api
from ..config import Config


class Command:
    def __init__(self, _args: Any, config: Config, env: Dict[str, str]):
        self.quiet: bool = False
        self.api = Api(config.api_url, env.get("USER"), config.install_id)

    def print(
        self,
        text: str,
        file_: TextIO = sys.stdout,
        end: str = "\n",
        force: bool = False,
    ) -> None:
        if not self.quiet or force:
            print(text, file=file_, end=end)

    def print_info(self, text: str) -> None:
        self.print(f"[0;32masciinema: {text}[0m")

    def print_warning(self, text: str) -> None:
        self.print(f"[0;33masciinema: {text}[0m")

    def print_error(self, text: str) -> None:
        self.print(
            f"[0;31masciinema: {text}[0m", file_=sys.stderr, force=True
        )
