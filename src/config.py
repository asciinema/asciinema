import os
import ConfigParser
import uuid

class Config:

    def __init__(self):
        self.base_dir_path = os.path.expanduser("~/.ascii.io")
        self.config_filename = '%s/config' % self.base_dir_path
        self.queue_dir_path = '%s/queue' % self.base_dir_path

        self.create_base_dir()
        self.parse_config_file()

    def create_base_dir(self):
        if not os.path.isdir(self.base_dir_path):
            os.mkdir(self.base_dir_path)

    def parse_config_file(self):
        config = ConfigParser.RawConfigParser()
        config.add_section('user')
        config.add_section('api')
        config.add_section('record')

        try:
            config.read(self.config_filename)
        except ConfigParser.ParsingError:
            print('Config file %s contains syntax errors' %
                    self.config_filename)
            sys.exit(2)

        self.config = config

    @property
    def api_url(self):
        try:
            api_url = self.config.get('api', 'url')
        except ConfigParser.NoOptionError:
            api_url = 'http://ascii.io'

        api_url = os.environ.get('ASCII_IO_API_URL', api_url)

        return api_url

    @property
    def user_token(self):
        try:
            user_token = self.config.get('user', 'token')
        except ConfigParser.NoOptionError:
            user_token = str(uuid.uuid1())
            self.config.set('user', 'token', user_token)

            with open(self.config_filename, 'wb') as f:
                self.config.write(f)

        return user_token
