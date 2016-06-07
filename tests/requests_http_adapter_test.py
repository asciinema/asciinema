import requests
from nose.tools import assert_equal
from .test_helper import Test
from asciinema.requests_http_adapter import RequestsHttpAdapter


class FakeResponse(object):

    def __init__(self, status=200, headers={}, body=''):
        self.status_code = status
        self.headers = headers
        self.text = body


class TestRequestsHttpAdapter(Test):

    def setUp(self):
        Test.setUp(self)
        self._real_requests_post = requests.post
        requests.post = self._fake_post

    def tearDown(self):
        Test.tearDown(self)
        requests.post = self._real_requests_post

    def test_post(self):
        adapter = RequestsHttpAdapter()

        status, headers, body = adapter.post(
            'http://the/url',
            { 'field': 'value' },
            { 'file': ('name.txt', b'contents') },
            { 'foo': 'bar' }
        )

        assert_equal('http://the/url', self._post_args['url'])
        assert_equal({ 'field': 'value' }, self._post_args['data'])
        assert_equal({ 'file': ('name.txt', b'contents') }, self._post_args['files'])
        assert_equal({ 'foo': 'bar' }, self._post_args['headers'])

        assert_equal(200, status)
        assert_equal({ 'Content-type': 'text/plain' }, headers)
        assert_equal('body', body)

    def _fake_post(self, url, data={}, files={}, headers={}):
        self._post_args = { 'url': url, 'data': data, 'files': files,
                            'headers': headers }

        return FakeResponse(200, { 'Content-type': 'text/plain' }, 'body' )
