from constants import SCRIPT_NAME

TEXT = '''usage: %s [-h] [-i] [-y] [-c <command>] [-t <title>] [action]

Asciicast recorder+uploader.

Actions:
 rec           record asciicast (this is the default when no action given)
 upload        upload recorded (but not uploaded) asciicasts
 auth          authenticate and/or claim recorded asciicasts

Optional arguments:
 -c command    run specified command instead of shell ($SHELL)
 -t title      specify title of recorded asciicast
 -y            don't prompt for confirmation
 -h, --help    show this help message and exit
 --version     show version information''' % SCRIPT_NAME
