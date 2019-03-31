import os
import os.path as path
import sys
import uuid
import configparser


class ConfigError(Exception):
    pass


DEFAULT_API_URL = 'https://asciinema.org'
DEFAULT_RECORD_ENV = 'SHELL,TERM'


class Config:

    def __init__(self, config_home, env=None):
        self.config_home = config_home
        self.config_file_path = path.join(config_home, "config")
        self.install_id_path = path.join(self.config_home, 'install-id')
        self.config = configparser.ConfigParser()
        self.config.read(self.config_file_path)
        self.env = env if env is not None else os.environ

    def upgrade(self):
        try:
            self.install_id
        except ConfigError:
            id = self.__api_token() or self.__user_token() or self.__gen_install_id()
            self.__save_install_id(id)

            items = {name: dict(section) for (name, section) in self.config.items()}
            if items == {'DEFAULT': {}, 'api': {'token': id}} or items == {'DEFAULT': {}, 'user': {'token': id}}:
                os.remove(self.config_file_path)

        if self.env.get('ASCIINEMA_API_TOKEN'):
            raise ConfigError('ASCIINEMA_API_TOKEN variable is no longer supported, please use ASCIINEMA_INSTALL_ID instead')

    def __read_install_id(self):
        p = self.install_id_path
        if path.isfile(p):
            with open(p, 'r') as f:
                return f.read().strip()

    def __gen_install_id(self):
        return str(uuid.uuid4())

    def __save_install_id(self, id):
        self.__create_config_home()

        with open(self.install_id_path, 'w') as f:
            f.write(id)

    def __create_config_home(self):
        if not path.exists(self.config_home):
            os.makedirs(self.config_home)

    def __api_token(self):
        try:
            return self.config.get('api', 'token')
        except (configparser.NoOptionError, configparser.NoSectionError):
            pass

    def __user_token(self):
        try:
            return self.config.get('user', 'token')
        except (configparser.NoOptionError, configparser.NoSectionError):
            pass

    @property
    def install_id(self):
        id = self.env.get('ASCIINEMA_INSTALL_ID') or self.__read_install_id()

        if id:
            return id
        else:
            raise ConfigError('no install ID found')

    @property
    def api_url(self):
        return self.env.get(
            'ASCIINEMA_API_URL',
            self.config.get('api', 'url', fallback=DEFAULT_API_URL)
        )

    @property
    def record_stdin(self):
        return self.config.getboolean('record', 'stdin', fallback=False)

    @property
    def record_command(self):
        return self.config.get('record', 'command', fallback=None)

    @property
    def record_env(self):
        return self.config.get('record', 'env', fallback=DEFAULT_RECORD_ENV)

    @property
    def record_idle_time_limit(self):
        fallback = self.config.getfloat('record', 'maxwait', fallback=None)  # pre 2.0
        return self.config.getfloat('record', 'idle_time_limit', fallback=fallback)

    @property
    def record_yes(self):
        return self.config.getboolean('record', 'yes', fallback=False)

    @property
    def record_quiet(self):
        return self.config.getboolean('record', 'quiet', fallback=False)

    @property
    def play_idle_time_limit(self):
        fallback = self.config.getfloat('play', 'maxwait', fallback=None)  # pre 2.0
        return self.config.getfloat('play', 'idle_time_limit', fallback=fallback)

    @property
    def play_speed(self):
        return self.config.getfloat('play', 'speed', fallback=1.0)

    @property
    def notifications_enabled(self):
        return self.config.getboolean('notifications', 'enabled', fallback=True)

    @property
    def notifications_command(self):
        return self.config.get('notifications', 'command', fallback=None)


def get_config_home(env=os.environ):
    env_asciinema_config_home = env.get("ASCIINEMA_CONFIG_HOME")
    env_xdg_config_home = env.get("XDG_CONFIG_HOME")
    env_home = env.get("HOME")

    config_home = None

    if env_asciinema_config_home:
        config_home = env_asciinema_config_home
    elif env_xdg_config_home:
        config_home = path.join(env_xdg_config_home, "asciinema")
    elif env_home:
        if path.isfile(path.join(env_home, ".asciinema", "config")):
            # location for versions < 1.1
            config_home = path.join(env_home, ".asciinema")
        else:
            config_home = path.join(env_home, ".config", "asciinema")
    else:
        raise Exception("need $HOME or $XDG_CONFIG_HOME or $ASCIINEMA_CONFIG_HOME")

    return config_home


def load(env=os.environ):
    config = Config(get_config_home(env), env)
    config.upgrade()
    return config
