from nose.tools import assert_equal

import os
import tempfile
import re

from asciinema.config import Config


def create_config(content=None, overrides={}):
    dir = tempfile.mkdtemp()
    path = dir + '/config'

    if content:
        with open(path, 'w') as f:
            f.write(content)

    return Config(path, overrides)


class TestConfig(object):

    def test_api_url_when_no_file_and_no_override_set(self):
        config = create_config()
        assert_equal('http://asciinema.org', config.api_url)

    def test_api_url_when_no_url_set_and_no_override_set(self):
        config = create_config('')
        assert_equal('http://asciinema.org', config.api_url)

    def test_api_url_when_url_set_and_no_override_set(self):
        config = create_config("[api]\nurl = http://the/url")
        assert_equal('http://the/url', config.api_url)

    def test_api_url_when_url_set_and_override_set(self):
        config = create_config("[api]\nurl = http://the/url", {
            'ASCIINEMA_API_URL': 'http://the/url2' })
        assert_equal('http://the/url2', config.api_url)

    def test_user_token_when_no_file(self):
        config = create_config()

        assert re.match('^\w{8}-\w{4}-\w{4}-\w{4}-\w{12}', config.user_token)
        assert os.path.isfile(config.path)

    def test_user_token_when_no_dir(self):
        config = create_config()
        dir = os.path.dirname(config.path)
        os.rmdir(dir)

        assert re.match('^\w{8}-\w{4}-\w{4}-\w{4}-\w{12}', config.user_token)
        assert os.path.isfile(config.path)

    def test_user_token_when_no_token_set(self):
        config = create_config('')
        assert re.match('^\w{8}-\w{4}-\w{4}-\w{4}-\w{12}', config.user_token)

    def test_user_token_when_token_set(self):
        token = 'foo-bar-baz'
        config = create_config("[user]\ntoken = %s" % token)
        assert re.match(token, config.user_token)
