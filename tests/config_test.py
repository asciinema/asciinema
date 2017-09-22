from nose.tools import assert_equal, assert_raises

import os
import tempfile
import re

import asciinema.config as cfg


def create_config(content='', env={}):
    dir = tempfile.mkdtemp()
    path = dir + '/config'

    with open(path, 'w') as f:
        f.write(content)

    return cfg.Config(cfg.load_file([path]), env)


def test_load_config():
    with tempfile.TemporaryDirectory() as dir:
        config = cfg.load({'ASCIINEMA_CONFIG_HOME': dir + '/foo/bar'})
        assert re.match('^\w{8}-\w{4}-\w{4}-\w{4}-\w{12}', config.api_token)

        with open(dir + '/config', 'w') as f:
            token = 'foo-bar-baz-qux-quux'
            f.write("[api]\ntoken = %s" % token)

        config = cfg.load({'ASCIINEMA_CONFIG_HOME': dir})
        assert_equal(token, config.api_token)


def test_default_api_url():
    config = create_config('')
    assert_equal('https://asciinema.org', config.api_url)


def test_default_record_command():
    config = create_config('')
    assert_equal(None, config.record_command)


def test_default_record_env():
    config = create_config('')
    assert_equal('SHELL,TERM', config.record_env)


def test_default_record_max_wait():
    config = create_config('')
    assert_equal(None, config.record_max_wait)


def test_default_record_yes():
    config = create_config('')
    assert_equal(False, config.record_yes)


def test_default_record_quiet():
    config = create_config('')
    assert_equal(False, config.record_quiet)


def test_default_play_max_wait():
    config = create_config('')
    assert_equal(None, config.play_max_wait)


def test_api_url():
    config = create_config("[api]\nurl = http://the/url")
    assert_equal('http://the/url', config.api_url)


def test_api_url_when_override_set():
    config = create_config("[api]\nurl = http://the/url", {
        'ASCIINEMA_API_URL': 'http://the/url2'})
    assert_equal('http://the/url2', config.api_url)


def test_api_token():
    token = 'foo-bar-baz'
    config = create_config("[api]\ntoken = %s" % token)
    assert re.match(token, config.api_token)


def test_api_token_when_no_api_token_set():
    config = create_config('')
    with assert_raises(Exception):
        config.api_token


def test_api_token_when_user_token_set():
    token = 'foo-bar-baz'
    config = create_config("[user]\ntoken = %s" % token)
    assert re.match(token, config.api_token)


def test_api_token_when_api_token_set_and_user_token_set():
    user_token = 'foo'
    api_token = 'bar'
    config = create_config("[user]\ntoken = %s\n[api]\ntoken = %s" % (user_token, api_token))
    assert re.match(api_token, config.api_token)


def test_record_command():
    command = 'bash -l'
    config = create_config("[record]\ncommand = %s" % command)
    assert_equal(command, config.record_command)


def test_record_env():
    config = create_config("[record]\nenv = FOO,BAR")
    assert_equal('FOO,BAR', config.record_env)


def test_record_max_wait():
    max_wait = '2.35'
    config = create_config("[record]\nmaxwait = %s" % max_wait)
    assert_equal(2.35, config.record_max_wait)


def test_record_yes():
    yes = 'yes'
    config = create_config("[record]\nyes = %s" % yes)
    assert_equal(True, config.record_yes)


def test_record_quiet():
    quiet = 'yes'
    config = create_config("[record]\nquiet = %s" % quiet)
    assert_equal(True, config.record_quiet)


def test_play_max_wait():
    max_wait = '2.35'
    config = create_config("[play]\nmaxwait = %s" % max_wait)
    assert_equal(2.35, config.play_max_wait)
