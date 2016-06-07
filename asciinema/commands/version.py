from asciinema import __version__


class VersionCommand(object):

    def execute(self):
        print('asciinema %s' % __version__)
