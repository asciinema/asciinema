import json
import bz2
import platform
import re

from asciinema import __version__
from .urllib_http_adapter import URLLibHttpAdapter


class ResourceNotFoundError(Exception):
    pass


class ServerMaintenanceError(Exception):
    pass


class Uploader(object):

    def __init__(self, http_adapter=None):
        self.http_adapter = http_adapter if http_adapter is not None else URLLibHttpAdapter()

    def upload(self, api_url, api_token, asciicast):
        url = '%s/api/asciicasts' % api_url
        files = self._asciicast_files(asciicast)
        headers = self._headers()

        status, headers, body = self.http_adapter.post(url, files=files,
                                                            headers=headers,
                                                            username=asciicast.username,
                                                            password=api_token)

        if status == 503:
            raise ServerMaintenanceError()

        if status == 404:
            raise ResourceNotFoundError()

        return body

    def _asciicast_files(self, asciicast):
        return {
            'asciicast[stdout]': self._stdout_data_file(asciicast.stdout),
            'asciicast[stdout_timing]': self._stdout_timing_file(asciicast.stdout),
            'asciicast[meta]': self._meta_file(asciicast)
        }

    def _headers(self):
        return { 'User-Agent': self._user_agent() }

    def _stdout_data_file(self, stdout):
        return ('stdout', bz2.compress(stdout.data))

    def _stdout_timing_file(self, stdout):
        return ('stdout.time', bz2.compress(stdout.timing))

    def _meta_file(self, asciicast):
        return ('meta.json', self._meta_json(asciicast))

    def _meta_json(self, asciicast):
        meta_data = asciicast.meta_data

        return json.dumps(meta_data).encode('utf-8')

    def _user_agent(self):
        os = re.sub('([^-]+)-(.*)', '\\1/\\2', platform.platform())

        return 'asciinema/%s %s/%s %s' % (__version__,
            platform.python_implementation(), platform.python_version(), os)
