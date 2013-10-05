import re

from asciinema.commands.auth import AuthCommand
from test_helper import assert_printed, Test


class TestAuthCommand(Test):

    def test_execute(self):
        command = AuthCommand('http://the/url', 'a1b2c3')

        command.execute()

        assert_printed('http://the/url/connect/a1b2c3')
