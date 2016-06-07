import json
import bz2
import platform
import re

from asciinema import __version__
from .requests_http_adapter import RequestsHttpAdapter


class ResourceNotFoundError(Exception):
    pass


class ServerMaintenanceError(Exception):
    pass


class Uploader(object):

    def __init__(self, http_adapter=None):
        self.http_adapter = http_adapter if http_adapter is not None else RequestsHttpAdapter()

    def upload(self, api_url, api_token, asciicast):
        url = '%s/api/asciicasts' % api_url
        files = self._asciicast_files(asciicast, api_token)
        headers = self._headers()

        status, headers, body = self.http_adapter.post(url, files=files,
                                                            headers=headers)

        if status == 503:
            raise ServerMaintenanceError()

        if status == 404:
            raise ResourceNotFoundError()

        return body

    def _asciicast_files(self, asciicast, api_token):
        return {
            'asciicast[stdout]': self._stdout_data_file(asciicast.stdout),
            'asciicast[stdout_timing]': self._stdout_timing_file(asciicast.stdout),
            'asciicast[meta]': self._meta_file(asciicast, api_token)
        }

    def _headers(self):
        return { 'User-Agent': self._user_agent() }

    def _stdout_data_file(self, stdout):
        return ('stdout', bz2.compress(stdout.data))

    def _stdout_timing_file(self, stdout):
        return ('stdout.time', bz2.compress(stdout.timing))

    def _meta_file(self, asciicast, api_token):
        return ('meta.json', self._meta_json(asciicast, api_token))

    def _meta_json(self, asciicast, api_token):
        meta_data = asciicast.meta_data
        auth_data = { 'user_token': api_token }
        data = dict(list(meta_data.items()) + list(auth_data.items()))

        return json.dumps(data)

    def _user_agent(self):
        os = re.sub('([^-]+)-(.*)', '\\1/\\2', platform.platform())

        return 'asciinema/%s %s/%s %s' % (__version__,
            platform.python_implementation(), platform.python_version(), os)
