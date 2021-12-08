import sys
try:
    from StringIO import StringIO
except ImportError:
    from io import StringIO

import unittest

stdout = None


class Test(unittest.TestCase):

    def setUp(self):
        global stdout
        self.real_stdout = sys.stdout
        sys.stdout = stdout = StringIO()

    def tearDown(self):
        sys.stdout = self.real_stdout
