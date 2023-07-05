import codecs
import http
import io
import sys
from base64 import b64encode
from http.client import HTTPResponse
from typing import Any, Dict, Generator, Optional, Tuple
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen
from uuid import uuid4

from .http_adapter import HTTPConnectionError


class MultipartFormdataEncoder:
    def __init__(self) -> None:
        self.boundary = uuid4().hex
        self.content_type = f"multipart/form-data; boundary={self.boundary}"

    @classmethod
    def u(cls, s: Any) -> Any:
        if sys.hexversion >= 0x03000000 and isinstance(s, bytes):
            s = s.decode("utf-8")
        return s

    def iter(
        self, fields: Dict[str, Any], files: Dict[str, Tuple[str, Any]]
    ) -> Generator[Tuple[bytes, int], None, None]:
        """
        fields: {name: value} for regular form fields.
        files: {name: (filename, file-type)} for data to be uploaded as files

        yield body's chunk as bytes
        """
        encoder = codecs.getencoder("utf-8")
        for key, value in fields.items():
            key = self.u(key)
            yield encoder(f"--{self.boundary}\r\n")
            yield encoder(
                self.u(f'content-disposition: form-data; name="{key}"\r\n')
            )
            yield encoder("\r\n")
            if isinstance(value, (int, float)):
                value = str(value)
            yield encoder(self.u(value))
            yield encoder("\r\n")
        for key, filename_and_f in files.items():
            filename, f = filename_and_f
            key = self.u(key)
            filename = self.u(filename)
            yield encoder(f"--{self.boundary}\r\n")
            yield encoder(
                self.u(
                    "content-disposition: form-data"
                    f'; name="{key}"'
                    f'; filename="{filename}"\r\n'
                )
            )
            yield encoder("content-type: application/octet-stream\r\n")
            yield encoder("\r\n")
            data = f.read()
            yield (data, len(data))
            yield encoder("\r\n")
        yield encoder(f"--{self.boundary}--\r\n")

    def encode(
        self, fields: Dict[str, Any], files: Dict[str, Tuple[str, Any]]
    ) -> Tuple[str, bytes]:
        body = io.BytesIO()
        for chunk, _ in self.iter(fields, files):
            body.write(chunk)
        return self.content_type, body.getvalue()


class URLLibHttpAdapter:  # pylint: disable=too-few-public-methods
    def post(  # pylint: disable=too-many-arguments,too-many-locals
        self,
        url: str,
        fields: Optional[Dict[str, Any]] = None,
        files: Optional[Dict[str, Tuple[str, Any]]] = None,
        headers: Optional[Dict[str, str]] = None,
        username: Optional[str] = None,
        password: Optional[str] = None,
    ) -> Tuple[Any, Optional[Dict[str, str]], bytes]:
        # avoid dangerous mutable default arguments
        if fields is None:
            fields = {}
        if files is None:
            files = {}
        if headers is None:
            headers = {}

        content_type, body = MultipartFormdataEncoder().encode(fields, files)

        headers = headers.copy()
        headers["content-type"] = content_type

        if password:
            encoded_auth = b64encode(
                f"{username}:{password}".encode("utf_8")
            ).decode("utf_8")
            headers["authorization"] = f"Basic {encoded_auth}"

        request = Request(url, data=body, headers=headers, method="POST")

        try:
            with urlopen(request) as response:
                status = response.status
                headers = self._parse_headers(response)
                body = response.read().decode("utf-8")
        except HTTPError as e:
            status = e.code
            headers = {}
            body = e.read()
        except (http.client.RemoteDisconnected, URLError) as e:
            raise HTTPConnectionError(str(e)) from e

        return (status, headers, body)

    @staticmethod
    def _parse_headers(response: HTTPResponse) -> Dict[str, str]:
        headers = {k.lower(): v for k, v in response.getheaders()}

        return headers
