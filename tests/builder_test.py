from nose.tools import assert_equal

from asciinema.commands.builder import get_command
from asciinema.commands.error import ErrorCommand
from asciinema.commands.record import RecordCommand
from asciinema.commands.auth import AuthCommand
from asciinema.commands.help import HelpCommand
from asciinema.commands.version import VersionCommand


class Config(object):

    def api_url(self):
        return 'http://api/url'

    def user_token(self):
        return 'a-toh-can'


class TestGetCommand(object):

    def setUp(self):
        self.config = Config()

    def test_get_command_when_cmd_is_absent(self):
        command = get_command([], self.config)

        assert_equal(RecordCommand, type(command))

    def test_get_command_when_cmd_is_rec(self):
        command = get_command(['rec'], self.config)

        assert_equal(RecordCommand, type(command))
        assert_equal(self.config.api_url, command.api_url)
        assert_equal(self.config.user_token, command.user_token)
        assert_equal(None, command.cmd)
        assert_equal(None, command.title)
        assert_equal(False, command.skip_confirmation)

    def test_get_command_when_cmd_is_rec_and_options_given(self):
        argv = ['-c', '/bin/bash -l', '-t', "O'HAI LOL", '-y', 'rec']
        command = get_command(argv, self.config)

        assert_equal(RecordCommand, type(command))
        assert_equal(self.config.api_url, command.api_url)
        assert_equal(self.config.user_token, command.user_token)
        assert_equal('/bin/bash -l', command.cmd)
        assert_equal("O'HAI LOL", command.title)
        assert_equal(True, command.skip_confirmation)

    def test_get_command_when_cmd_is_auth(self):
        command = get_command(['auth'], self.config)

        assert_equal(AuthCommand, type(command))
        assert_equal(self.config.api_url, command.api_url)
        assert_equal(self.config.user_token, command.user_token)

    def test_get_command_when_options_include_h(self):
        command = get_command(['-h'], self.config)

        assert_equal(HelpCommand, type(command))

    def test_get_command_when_options_include_help(self):
        command = get_command(['--help'], self.config)

        assert_equal(HelpCommand, type(command))

    def test_get_command_when_options_include_v(self):
        command = get_command(['-v'], self.config)

        assert_equal(VersionCommand, type(command))

    def test_get_command_when_options_include_version(self):
        command = get_command(['--version'], self.config)

        assert_equal(VersionCommand, type(command))

    def test_get_command_when_cmd_is_unknown(self):
        command = get_command(['foobar'], self.config)

        assert_equal(ErrorCommand, type(command))
        assert_equal("'foobar' is not an asciinema command", command.message)

    def test_get_command_when_too_many_cmds(self):
        command = get_command(['foo', 'bar'], self.config)

        assert_equal(ErrorCommand, type(command))
        assert_equal("Too many arguments", command.message)
