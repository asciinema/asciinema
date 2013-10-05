from asciinema.common import VERSION


class VersionCommand(object):

    def execute(self):
        print 'asciinema %s' % VERSION
