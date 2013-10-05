class HelpCommand(object):

    def execute(self):
        print HELP_TEXT


HELP_TEXT = '''usage: asciinema [-h] [-y] [-c <command>] [-t <title>] [action]

Asciicast recorder+uploader.

Actions:
 rec              record asciicast (this is the default when no action given)
 auth             authenticate and/or claim recorded asciicasts

Optional arguments:
 -c command       run specified command instead of shell ($SHELL)
 -t title         specify title of recorded asciicast
 -y               don't prompt for confirmation
 -h, --help       show this help message and exit
 -v, --version    show version information'''
