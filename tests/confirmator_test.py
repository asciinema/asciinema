import sys

from asciinema.confirmator import Confirmator
from test_helper import assert_printed, assert_not_printed, Test


class FakeStdin(object):

    def set_line(self, line):
        self.line = line

    def readline(self):
        return self.line


class TestConfirmator(Test):

    def setUp(self):
        Test.setUp(self)
        self.real_stdin = sys.stdin
        sys.stdin = self.stdin = FakeStdin()

    def tearDown(self):
        Test.tearDown(self)
        sys.stdin = self.real_stdin

    def test_confirm_when_y_entered(self):
        confirmator = Confirmator()
        self.stdin.set_line("y\n")

        assert confirmator.confirm('Wanna?')
        assert_printed('Wanna?')

    def test_confirm_when_Y_entered(self):
        confirmator = Confirmator()
        self.stdin.set_line("Y\n")

        assert confirmator.confirm('Wanna?')
        assert_printed('Wanna?')

    def test_confirm_when_enter_hit(self):
        confirmator = Confirmator()
        self.stdin.set_line("\n")

        assert confirmator.confirm('Wanna?')
        assert_printed('Wanna?')

    def test_confirm_when_spaces_entered(self):
        confirmator = Confirmator()
        self.stdin.set_line("  \n")

        assert confirmator.confirm('Wanna?')
        assert_printed('Wanna?')

    def test_confirm_when_n_entered(self):
        confirmator = Confirmator()
        self.stdin.set_line("n\n")

        assert not confirmator.confirm('Wanna?')
        assert_printed('Wanna?')

    def test_confirm_when_foo_entered(self):
        confirmator = Confirmator()
        self.stdin.set_line("foo\n")

        assert not confirmator.confirm('Wanna?')
        assert_printed('Wanna?')
