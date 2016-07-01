import platform
import re

from asciinema import __version__
from asciinema.urllib_http_adapter import URLLibHttpAdapter
from asciinema.http_adapter import HTTPConnectionError


class APIError(Exception):
    pass

class Api:

    def __init__(self, url, user, token, http_adapter=None):
        self.url = url
        self.user = user
        self.token = token
        self.http_adapter = http_adapter if http_adapter is not None else URLLibHttpAdapter()

    def auth_url(self):
        return "{}/connect/{}".format(self.url, self.token)

    def upload_url(self):
        return "{}/api/asciicasts".format(self.url)

    def upload_asciicast(self, path):
        with open(path, 'rb') as f:
            try:
                status, headers, body = self.http_adapter.post(
                    self.upload_url(),
                    files={ "asciicast": ("asciicast.json", f) },
                    headers=self._headers(),
                    username=self.user,
                    password=self.token
                )
            except HTTPConnectionError as e:
                raise APIError(str(e))

        if status != 200 and status != 201:
            self._handle_error(status, body)

        warn = self._extract_warning_message(headers)

        return body, warn

    def _headers(self):
        return { 'User-Agent': self._user_agent() }

    def _user_agent(self):
        os = re.sub('([^-]+)-(.*)', '\\1/\\2', platform.platform())

        return 'asciinema/%s %s/%s %s' % (__version__,
            platform.python_implementation(), platform.python_version(), os)

    def _extract_warning_message(self, headers):
        pass # TODO

    def _handle_error(self, status, body):
        if status == 404:
            raise APIError("Your client version is no longer supported. Please upgrade to the latest version.")

        if status == 503:
            raise APIError("The server is down for maintenance. Try again in a minute.")
