class Command:

    def __init__(self, quiet=False):
        self.quiet = quiet

    def print(self, text):
        if not self.quiet:
            print(text)

    def print_info(self, text):
        if not self.quiet:
            print("\x1b[32m~ %s\x1b[0m" % text)

    def print_warning(self, text):
        if not self.quiet:
            print("\x1b[33m~ %s\x1b[0m" % text)
