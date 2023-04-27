import re
import tempfile
from os import path
from typing import Dict, Optional

import asciinema.config as cfg
from asciinema.config import Config


def create_config(
    content: Optional[str] = None, env: Optional[Dict[str, str]] = None
) -> Config:
    # avoid redefining `dir` builtin
    dir_ = tempfile.mkdtemp()

    if content:
        # avoid redefining `os.path`
        path_ = f"{dir_}/config"
        with open(path_, "wt", encoding="utf_8") as f:
            f.write(content)

    return cfg.Config(dir_, env)


def read_install_id(install_id_path: str) -> str:
    with open(install_id_path, "rt", encoding="utf_8") as f:
        return f.read().strip()


def test_upgrade_no_config_file() -> None:
    config = create_config()
    config.upgrade()
    install_id = read_install_id(config.install_id_path)

    assert re.match("^\\w{8}-\\w{4}-\\w{4}-\\w{4}-\\w{12}", install_id)
    assert install_id == config.install_id
    assert not path.exists(config.config_file_path)

    # it must not change after another upgrade

    config.upgrade()

    assert read_install_id(config.install_id_path) == install_id


def test_upgrade_config_file_with_api_token() -> None:
    config = create_config("[api]\ntoken = foo-bar-baz")
    config.upgrade()

    assert read_install_id(config.install_id_path) == "foo-bar-baz"
    assert config.install_id == "foo-bar-baz"
    assert not path.exists(config.config_file_path)

    config.upgrade()

    assert read_install_id(config.install_id_path) == "foo-bar-baz"


def test_upgrade_config_file_with_api_token_and_more() -> None:
    config = create_config(
        "[api]\ntoken = foo-bar-baz\nurl = http://example.com"
    )
    config.upgrade()

    assert read_install_id(config.install_id_path) == "foo-bar-baz"
    assert config.install_id == "foo-bar-baz"
    assert config.api_url == "http://example.com"
    assert path.exists(config.config_file_path)

    config.upgrade()

    assert read_install_id(config.install_id_path) == "foo-bar-baz"


def test_upgrade_config_file_with_user_token() -> None:
    config = create_config("[user]\ntoken = foo-bar-baz")
    config.upgrade()

    assert read_install_id(config.install_id_path) == "foo-bar-baz"
    assert config.install_id == "foo-bar-baz"
    assert not path.exists(config.config_file_path)

    config.upgrade()

    assert read_install_id(config.install_id_path) == "foo-bar-baz"


def test_upgrade_config_file_with_user_token_and_more() -> None:
    config = create_config(
        "[user]\ntoken = foo-bar-baz\n[api]\nurl = http://example.com"
    )
    config.upgrade()

    assert read_install_id(config.install_id_path) == "foo-bar-baz"
    assert config.install_id == "foo-bar-baz"
    assert config.api_url == "http://example.com"
    assert path.exists(config.config_file_path)

    config.upgrade()

    assert read_install_id(config.install_id_path) == "foo-bar-baz"


def test_default_api_url() -> None:
    config = create_config("")
    assert config.api_url == "https://asciinema.org"


def test_default_record_stdin() -> None:
    config = create_config("")
    assert config.record_stdin is False


def test_default_record_command() -> None:
    config = create_config("")
    assert config.record_command is None


def test_default_record_env() -> None:
    config = create_config("")
    assert config.record_env == "SHELL,TERM"


def test_default_record_idle_time_limit() -> None:
    config = create_config("")
    assert config.record_idle_time_limit is None


def test_default_record_yes() -> None:
    config = create_config("")
    assert config.record_yes is False


def test_default_record_quiet() -> None:
    config = create_config("")
    assert config.record_quiet is False


def test_default_play_idle_time_limit() -> None:
    config = create_config("")
    assert config.play_idle_time_limit is None


def test_api_url() -> None:
    config = create_config("[api]\nurl = http://the/url")
    assert config.api_url == "http://the/url"


def test_api_url_when_override_set() -> None:
    config = create_config(
        "[api]\nurl = http://the/url", {"ASCIINEMA_API_URL": "http://the/url2"}
    )
    assert config.api_url == "http://the/url2"


def test_record_command() -> None:
    command = "bash -l"
    config = create_config(f"[record]\ncommand = {command}")
    assert config.record_command == command


def test_record_stdin() -> None:
    config = create_config("[record]\nstdin = yes")
    assert config.record_stdin is True


def test_record_env() -> None:
    config = create_config("[record]\nenv = FOO,BAR")
    assert config.record_env == "FOO,BAR"


def test_record_idle_time_limit() -> None:
    config = create_config("[record]\nidle_time_limit = 2.35")
    assert config.record_idle_time_limit == 2.35

    config = create_config("[record]\nmaxwait = 2.35")
    assert config.record_idle_time_limit == 2.35


def test_record_yes() -> None:
    yes = "yes"
    config = create_config(f"[record]\nyes = {yes}")
    assert config.record_yes is True


def test_record_quiet() -> None:
    quiet = "yes"
    config = create_config(f"[record]\nquiet = {quiet}")
    assert config.record_quiet is True


def test_play_idle_time_limit() -> None:
    config = create_config("[play]\nidle_time_limit = 2.35")
    assert config.play_idle_time_limit == 2.35

    config = create_config("[play]\nmaxwait = 2.35")
    assert config.play_idle_time_limit == 2.35


def test_notifications_enabled() -> None:
    config = create_config("")
    assert config.notifications_enabled is True

    config = create_config("[notifications]\nenabled = yes")
    assert config.notifications_enabled is True

    config = create_config("[notifications]\nenabled = no")
    assert config.notifications_enabled is False


def test_notifications_command() -> None:
    config = create_config("")
    assert config.notifications_command is None

    config = create_config(
        '[notifications]\ncommand = tmux display-message "$TEXT"'
    )
    assert config.notifications_command == 'tmux display-message "$TEXT"'


def test_record_suppress_yes() -> None:
    yes = "yes"
    config = create_config(f"[record]\nsuppress_output = {yes}")
    assert config.suppress_output is True


def test_default_record_supress() -> None:
    config = create_config("")
    assert config.suppress_output is False
