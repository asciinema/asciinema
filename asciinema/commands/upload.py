from typing import Any

from ..api import APIError
from ..config import Config
from .command import Command


class UploadCommand(Command):
    def __init__(self, args: Any, config: Config, env: Any) -> None:
        Command.__init__(self, args, config, env)
        self.filename = args.filename

    def execute(self) -> int:
        try:
            result, warn = self.api.upload_asciicast(self.filename)

            if warn:
                self.print_warning(warn)

            self.print(result.get("message") or result["url"])

        except OSError as e:
            self.print_error(f"upload failed: {str(e)}")
            return 1

        except APIError as e:
            self.print_error(f"upload failed: {str(e)}")

            if e.retryable:
                self.print_error(
                    f"retry later by running: asciinema upload {self.filename}"
                )

            return 1

        return 0
