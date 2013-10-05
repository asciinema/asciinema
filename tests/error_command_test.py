from nose.tools import assert_raises

from asciinema.commands.error import ErrorCommand
from .test_helper import assert_printed, Test


class TestErrorCommand(Test):

    def test_execute(self):
        command = ErrorCommand('foo')

        assert_raises(SystemExit, command.execute)
        assert_printed('foo')
