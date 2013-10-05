import os
import sys

try:
    from ConfigParser import RawConfigParser, ParsingError, NoOptionError
except ImportError:
    from configparser import RawConfigParser, ParsingError, NoOptionError

import uuid


DEFAULT_CONFIG_FILE_PATH = "~/.asciinema/config"
DEFAULT_API_URL = 'http://asciinema.org'

class Config:

    def __init__(self, path=DEFAULT_CONFIG_FILE_PATH, overrides=None):
        self.path = os.path.expanduser(path)
        self.overrides = overrides if overrides is not None else os.environ

        self._parse_config_file()

    def _parse_config_file(self):
        config = RawConfigParser()
        config.add_section('user')
        config.add_section('api')

        try:
            config.read(self.path)
        except ParsingError:
            print('Config file %s contains syntax errors' % self.path)
            sys.exit(2)

        self.config = config

    @property
    def api_url(self):
        try:
            api_url = self.config.get('api', 'url')
        except NoOptionError:
            api_url = DEFAULT_API_URL

        api_url = self.overrides.get('ASCIINEMA_API_URL', api_url)

        return api_url

    @property
    def user_token(self):
        try:
            user_token = self.config.get('user', 'token')
        except NoOptionError:
            user_token = str(uuid.uuid1())
            self.config.set('user', 'token', user_token)

            self._ensure_base_dir()
            with open(self.path, 'w') as f:
                self.config.write(f)

        return user_token

    def _ensure_base_dir(self):
        dir = os.path.dirname(self.path)

        if not os.path.isdir(dir):
            os.mkdir(dir)
