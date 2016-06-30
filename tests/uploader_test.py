import json
import bz2
import platform
from nose.tools import assert_equal, assert_raises
from .test_helper import Test, FakeAsciicast
from asciinema import __version__
from asciinema.uploader import Uploader, ServerMaintenanceError, ResourceNotFoundError


class FakeHttpAdapter:

    def __init__(self, status):
        self.status = status
        self.url = None
        self.files = None
        self.headers = None

    def post(self, url, files, headers):
        self.url = url
        self.files = files
        self.headers = headers

        return (self.status, { 'Content-type': 'text/plain' }, b'success!')


class FakeStdout:

    def __init__(self, data=None, timing=None):
        self.data = data or b''
        self.timing = timing or b''


class TestUploader(Test):

    def setUp(self):
        Test.setUp(self)
        self.stdout = FakeStdout(b'data123', b'timing456')
        self.asciicast = FakeAsciicast(cmd='ls -l', title='tit',
                stdout=self.stdout, meta_data={ 'shell': '/bin/sh' })
        self.real_platform = platform.platform
        platform.platform = lambda: 'foo-bar-baz-qux-quux'

    def tearDown(self):
        Test.tearDown(self)
        platform.platform = self.real_platform

    def test_upload_when_status_201_returned(self):
        http_adapter = FakeHttpAdapter(201)
        uploader = Uploader(http_adapter)

        response_body = uploader.upload('http://api/url', 'a1b2c3', self.asciicast)

        assert_equal(b'success!', response_body)
        assert_equal('http://api/url/api/asciicasts', http_adapter.url)
        assert_equal(self._expected_files(), http_adapter.files)
        assert_equal(self._expected_headers(), http_adapter.headers)

    def test_upload_when_status_503_returned(self):
        http_adapter = FakeHttpAdapter(503)
        uploader = Uploader(http_adapter)

        assert_raises(ServerMaintenanceError, uploader.upload,
                      'http://api/url', 'a1b2c3', self.asciicast)

    def test_upload_when_status_404_returned(self):
        http_adapter = FakeHttpAdapter(404)
        uploader = Uploader(http_adapter)

        assert_raises(ResourceNotFoundError, uploader.upload,
                      'http://api/url', 'a1b2c3', self.asciicast)

    def _expected_files(self):
        return {
            'asciicast[meta]':
                ('meta.json', json.dumps({ 'shell': '/bin/sh',
                                           'user_token': 'a1b2c3' })),
            'asciicast[stdout]':
                ('stdout', bz2.compress(b'data123')),
            'asciicast[stdout_timing]':
                ('stdout.time', bz2.compress(b'timing456'))
        }

    def _expected_headers(self):
        return { 'User-Agent': 'asciinema/%s %s/%s %s' %
               (__version__, platform.python_implementation(),
                   platform.python_version(), 'foo/bar-baz-qux-quux') }
