import codecs
import mimetypes
import sys
import uuid
import io
import base64

from urllib.request import Request, urlopen
from urllib.error import HTTPError, URLError
from .http_adapter import HTTPConnectionError


class MultipartFormdataEncoder:
    def __init__(self):
        self.boundary = uuid.uuid4().hex
        self.content_type = 'multipart/form-data; boundary={}'.format(self.boundary)

    @classmethod
    def u(cls, s):
        if sys.hexversion >= 0x03000000 and isinstance(s, bytes):
            s = s.decode('utf-8')
        return s

    def iter(self, fields, files):
        """
        fields is a dict of {name: value} for regular form fields.
        files is a dict of {name: (filename, file-type)} for data to be uploaded as files
        Yield body's chunk as bytes
        """
        encoder = codecs.getencoder('utf-8')
        for (key, value) in fields.items():
            key = self.u(key)
            yield encoder('--{}\r\n'.format(self.boundary))
            yield encoder(self.u('Content-Disposition: form-data; name="{}"\r\n').format(key))
            yield encoder('\r\n')
            if isinstance(value, int) or isinstance(value, float):
                value = str(value)
            yield encoder(self.u(value))
            yield encoder('\r\n')
        for (key, filename_and_f) in files.items():
            filename, f = filename_and_f
            key = self.u(key)
            filename = self.u(filename)
            yield encoder('--{}\r\n'.format(self.boundary))
            yield encoder(self.u('Content-Disposition: form-data; name="{}"; filename="{}"\r\n').format(key, filename))
            yield encoder('Content-Type: {}\r\n'.format(mimetypes.guess_type(filename)[0] or 'application/octet-stream'))
            yield encoder('\r\n')
            data = f.read()
            yield (data, len(data))
            yield encoder('\r\n')
        yield encoder('--{}--\r\n'.format(self.boundary))

    def encode(self, fields, files):
        body = io.BytesIO()
        for chunk, chunk_len in self.iter(fields, files):
            body.write(chunk)
        return self.content_type, body.getvalue()


class URLLibHttpAdapter:

    def post(self, url, fields={}, files={}, headers={}, username=None, password=None):
        content_type, body = MultipartFormdataEncoder().encode(fields, files)

        headers = headers.copy()
        headers["Content-Type"] = content_type

        if password:
            auth = "%s:%s" % (username, password)
            encoded_auth = base64.encodestring(auth.encode('utf-8'))[:-1]
            headers["Authorization"] = b"Basic %" + encoded_auth

        request = Request(url, data=body, headers=headers, method="POST")

        try:
            response = urlopen(request)
            status = response.status
            headers = self._parse_headers(response)
            body = response.read().decode('utf-8')
        except HTTPError as e:
            status = e.code
            headers = {}
            body = e.read().decode('utf-8')
        except URLError as e:
            raise HTTPConnectionError(str(e))

        return (status, headers, body)

    def _parse_headers(self, response):
        headers = {}
        for k, v in response.getheaders():
            headers[k] = v

        return headers
