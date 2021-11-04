from typing import Any, Dict

from ..config import Config
from .command import Command


class AuthCommand(Command):
    def __init__(self, args: Any, config: Config, env: Dict[str, str]) -> None:
        Command.__init__(self, args, config, env)

    def execute(self) -> None:
        self.print(
            f"Open the following URL in a web browser to link your install ID "
            f"with your {self.api.hostname()} user account:\n\n"
            f"{self.api.auth_url()}\n\n"
            "This will associate all recordings uploaded from this machine "
            "(past and future ones) to your account"
            ", and allow you to manage them (change title/theme, delete) at "
            f"{self.api.hostname()}."
        )
