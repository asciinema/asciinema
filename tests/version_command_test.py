from asciinema.commands.version import VersionCommand
from asciinema.common import VERSION
from .test_helper import assert_printed, Test


class TestVersionCommand(Test):

    def test_execute(self):
        command = VersionCommand()

        command.execute()

        assert_printed('asciinema %s' % VERSION)
