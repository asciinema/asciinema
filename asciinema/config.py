import configparser
import os
from os import path
from typing import Any, Dict, Optional
from uuid import uuid4

DEFAULT_API_URL: str = "https://asciinema.org"
DEFAULT_RECORD_ENV: str = "SHELL,TERM"


class ConfigError(Exception):
    pass


class Config:
    def __init__(
        self,
        config_home: Any,
        env: Optional[Dict[str, str]] = None,
    ) -> None:
        self.config_home = config_home
        self.config_file_path = path.join(config_home, "config")
        self.install_id_path = path.join(self.config_home, "install-id")
        self.config = configparser.ConfigParser()
        self.config.read(self.config_file_path)
        self.env = env if env is not None else os.environ

    def upgrade(self) -> None:
        try:
            self.install_id
        except ConfigError:
            id_ = (
                self.__api_token()
                or self.__user_token()
                or self.__gen_install_id()
            )
            self.__save_install_id(id_)

            items = {
                name: dict(section) for (name, section) in self.config.items()
            }
            if items in (
                {"DEFAULT": {}, "api": {"token": id_}},
                {"DEFAULT": {}, "user": {"token": id_}},
            ):
                os.remove(self.config_file_path)

        if self.env.get("ASCIINEMA_API_TOKEN"):
            raise ConfigError(
                "ASCIINEMA_API_TOKEN variable is no longer supported"
                ", please use ASCIINEMA_INSTALL_ID instead"
            )

    def __read_install_id(self) -> Optional[str]:
        p = self.install_id_path
        if path.isfile(p):
            with open(p, "r", encoding="utf-8") as f:
                return f.read().strip()
        return None

    @staticmethod
    def __gen_install_id() -> str:
        return f"{uuid4()}"

    def __save_install_id(self, id_: str) -> None:
        self.__create_config_home()

        with open(self.install_id_path, "w", encoding="utf-8") as f:
            f.write(id_)

    def __create_config_home(self) -> None:
        if not path.exists(self.config_home):
            os.makedirs(self.config_home)

    def __api_token(self) -> Optional[str]:
        try:
            return self.config.get("api", "token")
        except (configparser.NoOptionError, configparser.NoSectionError):
            return None

    def __user_token(self) -> Optional[str]:
        try:
            return self.config.get("user", "token")
        except (configparser.NoOptionError, configparser.NoSectionError):
            return None

    @property
    def install_id(self) -> str:
        id_ = self.env.get("ASCIINEMA_INSTALL_ID") or self.__read_install_id()

        if id_:
            return id_
        raise ConfigError("no install ID found")

    @property
    def api_url(self) -> str:
        return self.env.get(
            "ASCIINEMA_API_URL",
            self.config.get("api", "url", fallback=DEFAULT_API_URL),
        )

    @property
    def record_stdin(self) -> bool:
        return self.config.getboolean("record", "stdin", fallback=False)

    @property
    def record_command(self) -> Optional[str]:
        return self.config.get("record", "command", fallback=None)

    @property
    def record_env(self) -> str:
        return self.config.get("record", "env", fallback=DEFAULT_RECORD_ENV)

    @property
    def record_idle_time_limit(self) -> Optional[float]:
        fallback = self.config.getfloat(
            "record", "maxwait", fallback=None
        )  # pre 2.0
        return self.config.getfloat(
            "record", "idle_time_limit", fallback=fallback
        )

    @property
    def record_yes(self) -> bool:
        return self.config.getboolean("record", "yes", fallback=False)

    @property
    def record_quiet(self) -> bool:
        return self.config.getboolean("record", "quiet", fallback=False)

    @property
    def record_prefix_key(self) -> Any:
        return self.__get_key("record", "prefix")

    @property
    def record_pause_key(self) -> Any:
        return self.__get_key("record", "pause", "C-\\")

    @property
    def record_add_marker_key(self) -> Any:
        return self.__get_key("record", "add_marker")

    @property
    def play_idle_time_limit(self) -> Optional[float]:
        fallback = self.config.getfloat(
            "play", "maxwait", fallback=None
        )  # pre 2.0
        return self.config.getfloat(
            "play", "idle_time_limit", fallback=fallback
        )

    @property
    def play_speed(self) -> float:
        return self.config.getfloat("play", "speed", fallback=1.0)

    @property
    def play_pause_key(self) -> Any:
        return self.__get_key("play", "pause", " ")

    @property
    def play_step_key(self) -> Any:
        return self.__get_key("play", "step", ".")

    @property
    def play_next_marker_key(self) -> Any:
        return self.__get_key("play", "next_marker", "]")

    @property
    def notifications_enabled(self) -> bool:
        return self.config.getboolean(
            "notifications", "enabled", fallback=True
        )

    @property
    def notifications_command(self) -> Optional[str]:
        return self.config.get("notifications", "command", fallback=None)

    def __get_key(self, section: str, name: str, default: Any = None) -> Any:
        key = self.config.get(section, f"{name}_key", fallback=default)

        if key:
            if len(key) == 3:
                upper_key = key.upper()

                if upper_key[0] == "C" and upper_key[1] == "-":
                    return bytes([ord(upper_key[2]) - 0x40])
                raise ConfigError(
                    f"invalid {name} key definition '{key}' - use"
                    f": {name}_key = C-x (with control key modifier)"
                    f", or {name}_key = x (with no modifier)"
                )
            return key.encode("utf-8")
        return None


def get_config_home(env: Any = None) -> Any:
    if env is None:
        env = os.environ
    env_asciinema_config_home = env.get("ASCIINEMA_CONFIG_HOME")
    env_xdg_config_home = env.get("XDG_CONFIG_HOME")
    env_home = env.get("HOME")

    config_home: Optional[str] = None

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
        raise Exception(
            "need $HOME or $XDG_CONFIG_HOME or $ASCIINEMA_CONFIG_HOME"
        )

    return config_home


def load(env: Any = None) -> Config:
    if env is None:
        env = os.environ
    config = Config(get_config_home(env), env)
    config.upgrade()
    return config
