import sys


class Command:
    """
    Prints information from commands

    Attribtes:
        text (str): text to be printed out
        file (File object): tells Command class where to print output
                            default: system's standard output.

        end (str): delimeter for newlines, default: '\n'
        force (bool): force print even if quiet is set to True, default: False
        quiet (bool): suppresses output if True, default: False

    """
    def __init__(self, quiet=False):

        self.quiet = quiet


    def print(self, text, file=sys.stdout, end="\n", force=False):
        """
        Custom print method that can change print destination and newline delimeter.

        Args:
            text (str): text to be printed
            file (File object): where to print output, default: standard output
            end (str): newline delimeter, default: '\n'
            force (bool): if True, forces method to print, default: False
        """
        if not self.quiet or force:
            print(text, file=file, end=end)

    # All of the methods below are similar but for different warning levels.
    def print_info(self, text):
        self.print("\x1b[0;32masciinema: %s\x1b[0m" % text)

    def print_warning(self, text):
        self.print("\x1b[0;33masciinema: %s\x1b[0m" % text)

    def print_error(self, text):
        self.print("\x1b[0;31masciinema: %s\x1b[0m" % text, file=sys.stderr, force=True)
