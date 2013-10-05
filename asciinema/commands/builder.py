import getopt

from .error import ErrorCommand
from .record import RecordCommand
from .auth import AuthCommand
from .help import HelpCommand
from .version import VersionCommand


def get_command(argv, config):
    try:
        opts, commands = getopt.getopt(argv, 'c:t:ihvy', ['help', 'version'])
    except getopt.error as msg:
        return ErrorCommand(msg)

    if len(commands) > 1:
        return ErrorCommand('Too many arguments')

    if len(commands) == 0:
        command = 'rec'
    elif len(commands) == 1:
        command = commands[0]

    cmd = None
    title = None
    skip_confirmation = False

    for opt, arg in opts:
        if opt in ('-h', '--help'):
            return HelpCommand()
        elif opt in('-v', '--version'):
            return VersionCommand()
        elif opt == '-c':
            cmd = arg
        elif opt == '-t':
            title = arg
        elif opt == '-y':
            skip_confirmation = True

    if command == 'rec':
        return RecordCommand(config.api_url, config.user_token, cmd, title,
                             skip_confirmation)
    elif command == 'auth':
        return AuthCommand(config.api_url, config.user_token)

    return ErrorCommand("'%s' is not an asciinema command" % command)
