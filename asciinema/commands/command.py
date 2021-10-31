import sys

from asciinema.api import Api


class Command:
    def __init__(self, args, config, env):
        self.quiet = False
        self.api = Api(config.api_url, env.get("USER"), config.install_id)

    def print(self, text, file=sys.stdout, end="\n", force=False):
        if not self.quiet or force:
            print(text, file=file, end=end)

    def print_info(self, text):
        self.print(f"[0;32masciinema: {text}[0m")

    def print_warning(self, text):
        self.print(f"[0;33masciinema: {text}[0m")

    def print_error(self, text):
        self.print(
            f"[0;31masciinema: {text}[0m", file=sys.stderr, force=True
        )
