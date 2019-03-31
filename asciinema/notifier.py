import os.path
import shutil
import subprocess


class Notifier():
    def is_available(self):
        return shutil.which(self.cmd) is not None

    def notify(self, text):
        subprocess.run(self.args(text), capture_output=True)
        # we don't want to print *ANYTHING* to the terminal
        # so we capture and ignore all output

    def get_icon_path(self):
        path = os.path.join(os.path.dirname(os.path.realpath(__file__)), "data/icon-256x256.png")

        if os.path.exists(path):
            return path


class AppleScriptNotifier(Notifier):
    cmd = "osascript"

    def args(self, text):
        text = text.replace('"', '\\"')
        return ['osascript', '-e', 'display notification "{}" with title "asciinema"'.format(text)]


class LibNotifyNotifier(Notifier):
    cmd = "notify-send"

    def args(self, text):
        icon_path = self.get_icon_path()

        if icon_path is not None:
            return ['notify-send', '-i', icon_path, 'asciinema', text]
        else:
            return ['notify-send', 'asciinema', text]


class TerminalNotifier(Notifier):
    cmd = "terminal-notifier"

    def args(self, text):
        icon_path = self.get_icon_path()

        if icon_path is not None:
            return ['terminal-notifier', '-title', 'asciinema', '-message', text, '-appIcon', icon_path]
        else:
            return ['terminal-notifier', '-title', 'asciinema', '-message', text]


class NoopNotifier():
    def notify(self, text):
        pass


def get_notifier():
    for c in [TerminalNotifier, AppleScriptNotifier, LibNotifyNotifier]:
        n = c()

        if n.is_available():
            return n

    return NoopNotifier()
