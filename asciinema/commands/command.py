class Command:

    def print(self, text):
        print(text)

    def print_info(self, text):
        print("\x1b[32m~ %s\x1b[0m" % text)

    def print_warning(self, text):
        print("\x1b[33m~ %s\x1b[0m" % text)
