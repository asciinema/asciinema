import json
import platform
import re
from typing import Any, Callable, Dict, Optional, Tuple, Union
from urllib.parse import urlparse

from . import __version__
from .http_adapter import HTTPConnectionError
from .urllib_http_adapter import URLLibHttpAdapter


class APIError(Exception):
    pass


class Api:
    def __init__(
        self,
        url: str,
        user: Optional[str],
        install_id: str,
        http_adapter: Any = None,
    ) -> None:
        self.url = url
        self.user = user
        self.install_id = install_id
        self.http_adapter = (
            http_adapter if http_adapter is not None else URLLibHttpAdapter()
        )

    def hostname(self) -> Optional[str]:
        return urlparse(self.url).hostname

    def auth_url(self) -> str:
        return f"{self.url}/connect/{self.install_id}"

    def upload_url(self) -> str:
        return f"{self.url}/api/asciicasts"

    def upload_asciicast(self, path_: str) -> Tuple[Any, Any]:
        with open(path_, "rb") as f:
            try:
                status, headers, body = self.http_adapter.post(
                    self.upload_url(),
                    files={"asciicast": ("ascii.cast", f)},
                    headers=self._headers(),
                    username=self.user,
                    password=self.install_id,
                )
            except HTTPConnectionError as e:
                raise APIError(str(e)) from e

        if status in (200, 201):
            self._handle_error(status, body)

        if (headers.get("content-type") or "")[0:16] == "application/json":
            result = json.loads(body)
        else:
            result = {"url": body}

        return result, headers.get("Warning")

    def _headers(self) -> Dict[str, Union[Callable[[], str], str]]:
        return {"user-agent": self._user_agent, "accept": "application/json"}

    @property
    @staticmethod
    def _user_agent() -> str:
        os = re.sub("([^-]+)-(.*)", "\\1/\\2", platform.platform())

        return (
            f"asciinema/{__version__} {platform.python_implementation()}"
            f"/{platform.python_version()} {os}"
        )

    @staticmethod
    def _handle_error(status: int, body: str) -> None:
        errors = {
            400: f"Invalid request: {body}",
            401: "Invalid or revoked install ID",
            404: (
                "API endpoint not found. "
                "This asciinema version may no longer be supported. "
                "Please upgrade to the latest version."
            ),
            413: "Sorry, your asciicast is too big.",
            422: f"Invalid asciicast: {body}",
            503: "The server is down for maintenance. Try again in a minute.",
        }

        error = errors.get(status)

        if not error:
            if status >= 500:
                error = (
                    "The server is having temporary problems. "
                    "Try again in a minute."
                )
            else:
                error = f"HTTP status: {status}"

        raise APIError(error)
