import json
import platform
import re
from urllib.parse import urlparse

from asciinema import __version__
from asciinema.http_adapter import HTTPConnectionError
from asciinema.urllib_http_adapter import URLLibHttpAdapter


class APIError(Exception):
    pass


class Api:
    def __init__(self, url, user, install_id, http_adapter=None):
        self.url = url
        self.user = user
        self.install_id = install_id
        self.http_adapter = (
            http_adapter if http_adapter is not None else URLLibHttpAdapter()
        )

    def hostname(self):
        return urlparse(self.url).hostname

    def auth_url(self):
        return "{}/connect/{}".format(self.url, self.install_id)

    def upload_url(self):
        return "{}/api/asciicasts".format(self.url)

    def upload_asciicast(self, path):
        with open(path, "rb") as f:
            try:
                status, headers, body = self.http_adapter.post(
                    self.upload_url(),
                    files={"asciicast": ("ascii.cast", f)},
                    headers=self._headers(),
                    username=self.user,
                    password=self.install_id,
                )
            except HTTPConnectionError as e:
                raise APIError(str(e))

        if status != 200 and status != 201:
            self._handle_error(status, body)

        if (headers.get("content-type") or "")[0:16] == "application/json":
            result = json.loads(body)
        else:
            result = {"url": body}

        return result, headers.get("Warning")

    def _headers(self):
        return {"User-Agent": self._user_agent(), "Accept": "application/json"}

    def _user_agent(self):
        os = re.sub("([^-]+)-(.*)", "\\1/\\2", platform.platform())

        return "asciinema/%s %s/%s %s" % (
            __version__,
            platform.python_implementation(),
            platform.python_version(),
            os,
        )

    def _handle_error(self, status, body):
        errors = {
            400: "Invalid request: %s" % body,
            401: "Invalid or revoked install ID",
            404: "API endpoint not found. This asciinema version may no longer be supported. Please upgrade to the latest version.",
            413: "Sorry, your asciicast is too big.",
            422: "Invalid asciicast: %s" % body,
            503: "The server is down for maintenance. Try again in a minute.",
        }

        error = errors.get(status)

        if not error:
            if status >= 500:
                error = "The server is having temporary problems. Try again in a minute."
            else:
                error = "HTTP status: %i" % status

        raise APIError(error)
