import shutil
import subprocess
from os import environ, path
from typing import Dict, List, Optional, Union


class Notifier:
    def __init__(self, cmd: str) -> None:
        self.cmd = cmd

    @staticmethod
    def get_icon_path() -> Optional[str]:
        path_ = path.join(
            path.dirname(path.realpath(__file__)),
            "data/icon-256x256.png",
        )

        if path.exists(path_):
            return path_
        return None

    def args(self, _text: str) -> List[str]:
        return ["/bin/sh", "-c", self.cmd]

    def is_available(self) -> bool:
        return shutil.which(self.cmd) is not None

    def notify(self, text: str) -> None:
        # We do not want to raise a `CalledProcessError` on command failure.
        # pylint: disable=subprocess-run-check
        # We do not want to print *ANYTHING* to the terminal
        # so we capture and ignore all output
        subprocess.run(self.args(text), capture_output=True)


class AppleScriptNotifier(Notifier):
    def __init__(self) -> None:
        super().__init__("osascript")

    def args(self, text: str) -> List[str]:
        text = text.replace('"', '\\"')
        return [
            self.cmd,
            "-e",
            f'display notification "{text}" with title "asciinema"',
        ]


class LibNotifyNotifier(Notifier):
    def __init__(self) -> None:
        super().__init__("notify-send")

    def args(self, text: str) -> List[str]:
        icon_path = self.get_icon_path()

        if icon_path is not None:
            return [self.cmd, "-i", icon_path, "asciinema", text]
        return [self.cmd, "asciinema", text]


class TerminalNotifier(Notifier):
    def __init__(self) -> None:
        super().__init__("terminal-notifier")

    def args(self, text: str) -> List[str]:
        icon_path = self.get_icon_path()

        if icon_path is not None:
            return [
                "terminal-notifier",
                "-title",
                "asciinema",
                "-message",
                text,
                "-appIcon",
                icon_path,
            ]
        return [
            "terminal-notifier",
            "-title",
            "asciinema",
            "-message",
            text,
        ]


class CustomCommandNotifier(Notifier):
    def env(self, text: str) -> Dict[str, str]:
        icon_path = self.get_icon_path()
        env = environ.copy()
        env["TEXT"] = text
        if icon_path is not None:
            env["ICON_PATH"] = icon_path
        return env

    def notify(self, text: str) -> None:
        # We do not want to raise a `CalledProcessError` on command failure.
        # pylint: disable=subprocess-run-check
        subprocess.run(
            self.args(text), env=self.env(text), capture_output=True
        )


class NoopNotifier:  # pylint: disable=too-few-public-methods
    def notify(self, text: str) -> None:
        pass


def get_notifier(
    enabled: bool = True, command: Optional[str] = None
) -> Union[Notifier, NoopNotifier]:
    if enabled:
        if command:
            return CustomCommandNotifier(command)
        for c in [TerminalNotifier, AppleScriptNotifier, LibNotifyNotifier]:
            n = c()

            if n.is_available():
                return n

    return NoopNotifier()
