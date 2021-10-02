from nose.tools import assert_equal, assert_raises

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


def test_upgrade_no_config_file():
    config = create_config()
    config.upgrade()
    install_id = read_install_id(config.install_id_path)

    assert re.match('^\\w{8}-\\w{4}-\\w{4}-\\w{4}-\\w{12}', install_id)
    assert_equal(install_id, config.install_id)
    assert not path.exists(config.config_file_path)

    # it must not change after another upgrade

    config.upgrade()

    assert_equal(read_install_id(config.install_id_path), install_id)


def test_upgrade_config_file_with_api_token():
    config = create_config("[api]\ntoken = foo-bar-baz")
    config.upgrade()

    assert_equal(read_install_id(config.install_id_path), 'foo-bar-baz')
    assert_equal(config.install_id, 'foo-bar-baz')
    assert not path.exists(config.config_file_path)

    config.upgrade()

    assert_equal(read_install_id(config.install_id_path), 'foo-bar-baz')


def test_upgrade_config_file_with_api_token_and_more():
    config = create_config("[api]\ntoken = foo-bar-baz\nurl = http://example.com")
    config.upgrade()

    assert_equal(read_install_id(config.install_id_path), 'foo-bar-baz')
    assert_equal(config.install_id, 'foo-bar-baz')
    assert_equal(config.api_url, 'http://example.com')
    assert path.exists(config.config_file_path)

    config.upgrade()

    assert_equal(read_install_id(config.install_id_path), 'foo-bar-baz')


def test_upgrade_config_file_with_user_token():
    config = create_config("[user]\ntoken = foo-bar-baz")
    config.upgrade()

    assert_equal(read_install_id(config.install_id_path), 'foo-bar-baz')
    assert_equal(config.install_id, 'foo-bar-baz')
    assert not path.exists(config.config_file_path)

    config.upgrade()

    assert_equal(read_install_id(config.install_id_path), 'foo-bar-baz')


def test_upgrade_config_file_with_user_token_and_more():
    config = create_config("[user]\ntoken = foo-bar-baz\n[api]\nurl = http://example.com")
    config.upgrade()

    assert_equal(read_install_id(config.install_id_path), 'foo-bar-baz')
    assert_equal(config.install_id, 'foo-bar-baz')
    assert_equal(config.api_url, 'http://example.com')
    assert path.exists(config.config_file_path)

    config.upgrade()

    assert_equal(read_install_id(config.install_id_path), 'foo-bar-baz')


def test_default_api_url():
    config = create_config('')
    assert_equal('https://asciinema.org', config.api_url)


def test_default_record_stdin():
    config = create_config('')
    assert_equal(False, config.record_stdin)


def test_default_record_command():
    config = create_config('')
    assert_equal(None, config.record_command)


def test_default_record_env():
    config = create_config('')
    assert_equal('SHELL,TERM', config.record_env)


def test_default_record_idle_time_limit():
    config = create_config('')
    assert_equal(None, config.record_idle_time_limit)


def test_default_record_yes():
    config = create_config('')
    assert_equal(False, config.record_yes)


def test_default_record_quiet():
    config = create_config('')
    assert_equal(False, config.record_quiet)


def test_default_play_idle_time_limit():
    config = create_config('')
    assert_equal(None, config.play_idle_time_limit)


def test_api_url():
    config = create_config("[api]\nurl = http://the/url")
    assert_equal('http://the/url', config.api_url)


def test_api_url_when_override_set():
    config = create_config("[api]\nurl = http://the/url", {
        'ASCIINEMA_API_URL': 'http://the/url2'})
    assert_equal('http://the/url2', config.api_url)


def test_record_command():
    command = 'bash -l'
    config = create_config("[record]\ncommand = %s" % command)
    assert_equal(command, config.record_command)


def test_record_stdin():
    config = create_config("[record]\nstdin = yes")
    assert_equal(True, config.record_stdin)


def test_record_env():
    config = create_config("[record]\nenv = FOO,BAR")
    assert_equal('FOO,BAR', config.record_env)


def test_record_idle_time_limit():
    config = create_config("[record]\nidle_time_limit = 2.35")
    assert_equal(2.35, config.record_idle_time_limit)

    config = create_config("[record]\nmaxwait = 2.35")
    assert_equal(2.35, config.record_idle_time_limit)


def test_record_yes():
    yes = 'yes'
    config = create_config("[record]\nyes = %s" % yes)
    assert_equal(True, config.record_yes)


def test_record_quiet():
    quiet = 'yes'
    config = create_config("[record]\nquiet = %s" % quiet)
    assert_equal(True, config.record_quiet)


def test_play_idle_time_limit():
    config = create_config("[play]\nidle_time_limit = 2.35")
    assert_equal(2.35, config.play_idle_time_limit)

    config = create_config("[play]\nmaxwait = 2.35")
    assert_equal(2.35, config.play_idle_time_limit)


def test_notifications_enabled():
    config = create_config('')
    assert_equal(True, config.notifications_enabled)

    config = create_config("[notifications]\nenabled = yes")
    assert_equal(True, config.notifications_enabled)

    config = create_config("[notifications]\nenabled = no")
    assert_equal(False, config.notifications_enabled)


def test_notifications_command():
    config = create_config('')
    assert_equal(None, config.notifications_command)

    config = create_config('[notifications]\ncommand = tmux display-message "$TEXT"')
    assert_equal('tmux display-message "$TEXT"', config.notifications_command)
