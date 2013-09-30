import sys
from StringIO import StringIO


stdout = None


def assert_printed(expected):
    if isinstance(expected, basestring):
        success = expected in stdout.getvalue()
        assert success, 'expected text "%s" not printed' % expected
    else:
        success = expected.match(stdout.getvalue())
        assert success, 'expected pattern "%s" not printed' % expected.pattern


def assert_not_printed(expected):
    success = expected not in stdout.getvalue()
    assert success, 'not expected text "%s" printed' % expected


class Test(object):

    def setUp(self):
        global stdout
        self.real_stdout = sys.stdout
        sys.stdout = stdout = StringIO()

    def tearDown(self):
        sys.stdout = self.real_stdout


class FakeClock(object):

    def __init__(self, values):
        self.values = values
        self.n = 0

    def time(self):
        value = self.values[self.n]
        self.n += 1

        return value
