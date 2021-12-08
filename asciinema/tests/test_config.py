import unittest

import os
import os.path as path
import tempfile
import re

import asciinema.config as cfg


def create_config(content=None, env={}):
    dir = tempfile.mkdtemp()

    if content:
        path = dir + '/config'
        with open(path, 'w') as f:
            f.write(content)

    return cfg.Config(dir, env)


def read_install_id(install_id_path):
    with open(install_id_path, 'r') as f:
        return f.read().strip()


class AsciinemaTests(unittest.TestCase):

    def test_upgrade_no_config_file(self):
        config = create_config()
        config.upgrade()
        install_id = read_install_id(config.install_id_path)

        self.assertIsNotNone(re.match('^\\w{8}-\\w{4}-\\w{4}-\\w{4}-\\w{12}', install_id))
        self.assertEqual(install_id, config.install_id)
        self.assertFalse(path.exists(config.config_file_path))

        # it must not change after another upgrade

        config.upgrade()

        self.assertEqual(read_install_id(config.install_id_path), install_id)

    def test_upgrade_config_file_with_api_token(self):
        config = create_config("[api]\ntoken = foo-bar-baz")
        config.upgrade()

        self.assertEqual(read_install_id(config.install_id_path), 'foo-bar-baz')
        self.assertEqual(config.install_id, 'foo-bar-baz')
        self.assertFalse(path.exists(config.config_file_path))

        config.upgrade()

        self.assertEqual(read_install_id(config.install_id_path), 'foo-bar-baz')

    def test_upgrade_config_file_with_api_token_and_more(self):
        config = create_config("[api]\ntoken = foo-bar-baz\nurl = http://example.com")
        config.upgrade()

        self.assertEqual(read_install_id(config.install_id_path), 'foo-bar-baz')
        self.assertEqual(config.install_id, 'foo-bar-baz')
        self.assertEqual(config.api_url, 'http://example.com')
        self.assertTrue(path.exists(config.config_file_path))

        config.upgrade()

        self.assertEqual(read_install_id(config.install_id_path), 'foo-bar-baz')

    def test_upgrade_config_file_with_user_token(self):
        config = create_config("[user]\ntoken = foo-bar-baz")
        config.upgrade()

        self.assertEqual(read_install_id(config.install_id_path), 'foo-bar-baz')
        self.assertEqual(config.install_id, 'foo-bar-baz')
        self.assertFalse(path.exists(config.config_file_path))

        config.upgrade()

        self.assertEqual(read_install_id(config.install_id_path), 'foo-bar-baz')

    def test_upgrade_config_file_with_user_token_and_more(self):
        config = create_config("[user]\ntoken = foo-bar-baz\n[api]\nurl = http://example.com")
        config.upgrade()

        self.assertEqual(read_install_id(config.install_id_path), 'foo-bar-baz')
        self.assertEqual(config.install_id, 'foo-bar-baz')
        self.assertEqual(config.api_url, 'http://example.com')
        self.assertTrue(path.exists(config.config_file_path))

        config.upgrade()

        self.assertEqual(read_install_id(config.install_id_path), 'foo-bar-baz')

    def test_default_api_url(self):
        config = create_config('')
        self.assertEqual('https://asciinema.org', config.api_url)

    def test_default_record_stdin(self):
        config = create_config('')
        self.assertEqual(False, config.record_stdin)

    def test_default_record_command(self):
        config = create_config('')
        self.assertEqual(None, config.record_command)

    def test_default_record_env(self):
        config = create_config('')
        self.assertEqual('SHELL,TERM', config.record_env)

    def test_default_record_idle_time_limit(self):
        config = create_config('')
        self.assertEqual(None, config.record_idle_time_limit)

    def test_default_record_yes(self):
        config = create_config('')
        self.assertEqual(False, config.record_yes)

    def test_default_record_quiet(self):
        config = create_config('')
        self.assertEqual(False, config.record_quiet)

    def test_default_play_idle_time_limit(self):
        config = create_config('')
        self.assertEqual(None, config.play_idle_time_limit)

    def test_api_url(self):
        config = create_config("[api]\nurl = http://the/url")
        self.assertEqual('http://the/url', config.api_url)

    def test_api_url_when_override_set(self):
        config = create_config("[api]\nurl = http://the/url", {
            'ASCIINEMA_API_URL': 'http://the/url2'})
        self.assertEqual('http://the/url2', config.api_url)

    def test_record_command(self):
        command = 'bash -l'
        config = create_config("[record]\ncommand = %s" % command)
        self.assertEqual(command, config.record_command)

    def test_record_stdin(self):
        config = create_config("[record]\nstdin = yes")
        self.assertEqual(True, config.record_stdin)

    def test_record_env(self):
        config = create_config("[record]\nenv = FOO,BAR")
        self.assertEqual('FOO,BAR', config.record_env)

    def test_record_idle_time_limit(self):
        config = create_config("[record]\nidle_time_limit = 2.35")
        self.assertEqual(2.35, config.record_idle_time_limit)

        config = create_config("[record]\nmaxwait = 2.35")
        self.assertEqual(2.35, config.record_idle_time_limit)

    def test_record_yes(self):
        yes = 'yes'
        config = create_config("[record]\nyes = %s" % yes)
        self.assertEqual(True, config.record_yes)

    def test_record_quiet(self):
        quiet = 'yes'
        config = create_config("[record]\nquiet = %s" % quiet)
        self.assertEqual(True, config.record_quiet)

    def test_play_idle_time_limit(self):
        config = create_config("[play]\nidle_time_limit = 2.35")
        self.assertEqual(2.35, config.play_idle_time_limit)

        config = create_config("[play]\nmaxwait = 2.35")
        self.assertEqual(2.35, config.play_idle_time_limit)

    def test_notifications_enabled(self):
        config = create_config('')
        self.assertEqual(True, config.notifications_enabled)

        config = create_config("[notifications]\nenabled = yes")
        self.assertEqual(True, config.notifications_enabled)

        config = create_config("[notifications]\nenabled = no")
        self.assertEqual(False, config.notifications_enabled)

    def test_notifications_command(self):
        config = create_config('')
        self.assertEqual(None, config.notifications_command)

        config = create_config('[notifications]\ncommand = tmux display-message "$TEXT"')
        self.assertEqual('tmux display-message "$TEXT"', config.notifications_command)


if __name__ == "__main__":
    unittest.main()
