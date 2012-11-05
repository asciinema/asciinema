import sys
import getopt

class Options:

    def __init__(self, argv):
        self.action = None
        self.command = None
        self.title = None
        self.record_input = False
        self.always_yes = False

        try:
            opts, args = getopt.getopt(argv[1:], 'c:t:ihy', ['help', 'version'])
        except getopt.error as msg:
            print('%s: %s' % (argv[0], msg))
            print('Run "%s --help" for list of available options' % argv[0])
            sys.exit(2)

        if len(args) == 0:
            self.action = 'rec'
        elif len(args) == 1:
            self.action = args[0]
        elif len(args) > 1:
            print('Too many arguments')
            print('Run "%s --help" for list of available options' % argv[0])
            sys.exit(2)

        for opt, arg in opts:
            if opt in ('-h', '--help'):
                self.action = 'help'
            elif opt == '--version':
                self.action = 'version'
            elif opt == '-c':
                self.command = arg
            elif opt == '-t':
                self.title = arg
            elif opt == '-i':
                self.record_input = True
            elif opt == '-y':
                self.always_yes = True
