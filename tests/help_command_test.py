from asciinema.commands.help import HelpCommand
from .test_helper import assert_printed, Test


class TestHelpCommand(Test):

    def test_execute(self):
        command = HelpCommand()

        command.execute()

        assert_printed('asciinema')
        assert_printed('usage')
        assert_printed('rec')
        assert_printed('auth')
