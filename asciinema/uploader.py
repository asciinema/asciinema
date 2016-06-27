import json
import bz2
import platform
import re
import os

from asciinema import __version__
from .urllib_http_adapter import URLLibHttpAdapter


class ResourceNotFoundError(Exception):
    pass


class ServerMaintenanceError(Exception):
    pass


class Uploader(object):

    def __init__(self, http_adapter=None):
        self.http_adapter = http_adapter if http_adapter is not None else URLLibHttpAdapter()

    def upload(self, api_url, api_token, path):
        url = '%s/api/asciicasts' % api_url

        with open(path, 'rb') as f:
            files = { "asciicast": ("asciicast.json", f) }
            headers = self._headers()

            status, headers, body = self.http_adapter.post(url, files=files,
                                                                headers=headers,
                                                                username=os.environ.get("USER"),
                                                                password=api_token)

        if status == 503:
            raise ServerMaintenanceError()

        if status == 404:
            raise ResourceNotFoundError()

        return body

    def _headers(self):
        return { 'User-Agent': self._user_agent() }

    def _user_agent(self):
        os = re.sub('([^-]+)-(.*)', '\\1/\\2', platform.platform())

        return 'asciinema/%s %s/%s %s' % (__version__,
            platform.python_implementation(), platform.python_version(), os)
