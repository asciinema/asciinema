from nose.tools import assert_equal
from nose.tools import assert_not_equal
from nose.tools import assert_raises
from nose.tools import raises

import os
import tempfile
import re

from config import Config


def create_config(config_file_content=None):
    base_path = tempfile.mkdtemp()

    if config_file_content:
        with open(base_path + '/config', 'w') as f:
            f.write(config_file_content)

    return Config(base_path)


class TestConfig(object):

    def test_api_url(self):
        # defaults to http://ascii.io
        config = create_config()
        assert_equal('http://ascii.io', config.api_url)

        # uses api.url from config file
        config = create_config("[api]\nurl = bar")
        assert_equal('bar', config.api_url)

        # can be overriden by ASCII_IO_API_URL env var
        os.environ['ASCII_IO_API_URL'] = 'foo'
        assert_equal('foo', config.api_url)
        del os.environ['ASCII_IO_API_URL']

    def test_user_token(self):
        # generates and saves new token in config file
        config = create_config()
        user_token = config.user_token
        assert re.match('^\w{8}-\w{4}-\w{4}-\w{4}-\w{12}', user_token)
        assert os.path.isfile(config.base_path + '/config')

        # reads existing token from config file
        token = 'foo-bar-baz'
        config = create_config("[user]\ntoken = %s" % token)
        assert_equal(token, config.user_token)
