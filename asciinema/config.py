import os
import os.path as path
import sys
import uuid
import configparser


class ConfigError(Exception):
    pass


DEFAULT_API_URL = 'https://asciinema.org'


class Config:

    def __init__(self, config, env=None):
        self.config = config
        self.env = env if env is not None else os.environ

    @property
    def api_url(self):
        return self.env.get(
            'ASCIINEMA_API_URL',
            self.config.get('api', 'url', fallback=DEFAULT_API_URL)
        )

    @property
    def api_token(self):
        try:
            return self.env.get('ASCIINEMA_API_TOKEN') or self.config.get('api', 'token')
        except (configparser.NoOptionError, configparser.NoSectionError):
            try:
                return self.config.get('user', 'token')
            except (configparser.NoOptionError, configparser.NoSectionError):
                raise ConfigError('no API token found in config file, and ASCIINEMA_API_TOKEN is unset')

    @property
    def record_command(self):
        return self.config.get('record', 'command', fallback=None)

    @property
    def record_max_wait(self):
        return self.config.getfloat('record', 'maxwait', fallback=None)

    @property
    def record_yes(self):
        return self.config.getboolean('record', 'yes', fallback=False)

    @property
    def record_quiet(self):
        return self.config.getboolean('record', 'quiet', fallback=False)

    @property
    def play_max_wait(self):
        return self.config.getfloat('play', 'maxwait', fallback=None)

    @property
    def play_speed(self):
        return self.config.getfloat('play', 'speed', fallback=1.0)


def load_file(paths):
    config = configparser.ConfigParser()
    read_paths = config.read(paths)

    if read_paths:
        return config


def create_file(filename):
    config = configparser.ConfigParser()
    config['api'] = {}
    config['api']['token'] = str(uuid.uuid4())

    if not path.exists(path.dirname(filename)):
        os.makedirs(path.dirname(filename))

    with open(filename, 'w') as f:
        config.write(f)

    return config


def load(env=os.environ):
    paths = []

    asciinema_config_home = env.get("ASCIINEMA_CONFIG_HOME")
    xdg_config_home = env.get("XDG_CONFIG_HOME")
    home = env.get("HOME")

    if asciinema_config_home:
        paths.append(path.join(asciinema_config_home, "config"))
    elif xdg_config_home:
        paths.append(path.join(xdg_config_home, "asciinema", "config"))
    elif home:
        paths.append(path.join(home, ".asciinema", "config"))
        paths.append(path.join(home, ".config", "asciinema", "config"))
    else:
        raise Exception("need $ASCIINEMA_CONFIG_HOME or $XDG_CONFIG_HOME or $HOME")

    config = load_file(paths) or create_file(paths[-1])

    return Config(config, env)
