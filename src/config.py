import os
import ConfigParser
import uuid

class Config:

    def __init__(self):
        self.config_filename = os.path.expanduser('~/.ascii.io/config')

        config = ConfigParser.RawConfigParser()
        config.add_section('user')
        config.add_section('api')
        config.add_section('record')

        try:
            config.read(self.config_filename)
        except ConfigParser.ParsingError:
            print('Config file %s contains syntax errors' % self.config_filename)
            sys.exit(2)

        self.config = config

        # if not os.path.isdir(BASE_DIR):
        #     os.mkdir(BASE_DIR)

    def api_url(self):
        try:
            api_url = self.config.get('api', 'url')
        except ConfigParser.NoOptionError:
            api_url = 'http://ascii.io'

        api_url = os.environ.get('ASCII_IO_API_URL', api_url)

        return api_url

    def user_token(self):
        try:
            user_token = self.config.get('user', 'token')
        except ConfigParser.NoOptionError:
            user_token = str(uuid.uuid1())
            self.config.set('user', 'token', user_token)

            with open(self.config_filename, 'wb') as configfile:
                self.config.write(configfile)

        return user_token
