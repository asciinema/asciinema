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


class TestConfig:

    def test_api_url_when_no_file_and_no_override_set(self):
        config = create_config()
        assert_equal('https://asciinema.org', config.api_url)

    def test_api_url_when_no_url_set_and_no_override_set(self):
        config = create_config('')
        assert_equal('https://asciinema.org', config.api_url)

    def test_api_url_when_url_set_and_no_override_set(self):
        config = create_config("[api]\nurl = http://the/url")
        assert_equal('http://the/url', config.api_url)

    def test_api_url_when_url_set_and_override_set(self):
        config = create_config("[api]\nurl = http://the/url", {
            'ASCIINEMA_API_URL': 'http://the/url2' })
        assert_equal('http://the/url2', config.api_url)

    def test_api_token_when_no_file(self):
        config = create_config()

        assert re.match('^\w{8}-\w{4}-\w{4}-\w{4}-\w{12}', config.api_token)
        assert os.path.isfile(config.path)

    def test_api_token_when_no_dir(self):
        config = create_config()
        dir = os.path.dirname(config.path)
        os.rmdir(dir)

        assert re.match('^\w{8}-\w{4}-\w{4}-\w{4}-\w{12}', config.api_token)
        assert os.path.isfile(config.path)

    def test_api_token_when_no_api_token_set(self):
        config = create_config('')
        assert re.match('^\w{8}-\w{4}-\w{4}-\w{4}-\w{12}', config.api_token)

    def test_api_token_when_api_token_set(self):
        token = 'foo-bar-baz'
        config = create_config("[api]\ntoken = %s" % token)
        assert re.match(token, config.api_token)

    def test_api_token_when_api_token_set_as_user_token(self):
        token = 'foo-bar-baz'
        config = create_config("[user]\ntoken = %s" % token)
        assert re.match(token, config.api_token)

    def test_api_token_when_api_token_set_and_user_token_set(self):
        user_token = 'foo'
        api_token = 'bar'
        config = create_config("[user]\ntoken = %s\n[api]\ntoken = %s" %
                               (user_token, api_token))
        assert re.match(api_token, config.api_token)
