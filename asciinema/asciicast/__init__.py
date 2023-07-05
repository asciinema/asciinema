import codecs
import gzip
import os
import sys
import urllib.error
from codecs import StreamReader
from html.parser import HTMLParser
from io import BytesIO
from typing import Any, List, TextIO, Union
from urllib.parse import urlparse, urlunparse
from urllib.request import Request, urlopen

from . import v1, v2


class LoadError(Exception):
    pass


class Parser(HTMLParser):
    def __init__(self) -> None:
        HTMLParser.__init__(self)
        self.url = None

    def error(self, message: str) -> None:
        raise NotImplementedError(
            "subclasses of ParserBase must override error()"
            ", but HTMLParser does not"
        )

    def handle_starttag(self, tag: str, attrs: List[Any]) -> None:
        # look for <link rel="alternate"
        #                type="application/x-asciicast"
        #                href="https://...cast">
        if tag == "link":
            # avoid modifying function signature keyword args from base class
            _attrs = {}
            for k, v in attrs:
                _attrs[k] = v

            if _attrs.get("rel") == "alternate":
                type_ = _attrs.get("type")
                if type_ in (
                    "application/asciicast+json",
                    "application/x-asciicast",
                ):
                    self.url = _attrs.get("href")


def open_url(url: str) -> Union[StreamReader, TextIO]:
    if url == "-":
        return sys.stdin

    if url.startswith("ipfs://"):
        url = f"https://ipfs.io/ipfs/{url[7:]}"
    elif url.startswith("dweb:/ipfs/"):
        url = f"https://ipfs.io/{url[5:]}"

    if url.startswith("http:") or url.startswith("https:"):
        req = Request(url)
        req.add_header("Accept-Encoding", "gzip")
        body = None
        content_type = None
        utf8_reader = codecs.getreader("utf-8")

        with urlopen(req) as response:
            body = response
            content_type = response.headers["Content-Type"]
            url = response.geturl()  # final URL after redirects

            if response.headers["Content-Encoding"] == "gzip":
                body = gzip.open(body)

            body = BytesIO(body.read())

        if content_type and content_type.startswith("text/html"):
            html = utf8_reader(body, errors="replace").read()
            parser = Parser()
            parser.feed(html)
            new_url = parser.url

            if not new_url:
                raise LoadError(
                    '<link rel="alternate" '
                    'type="application/x-asciicast" '
                    'href="..."> '
                    "not found in fetched HTML document"
                )

            if "://" not in new_url:
                base_url = urlparse(url)

                if new_url.startswith("/"):
                    new_url = urlunparse(
                        (base_url[0], base_url[1], new_url, "", "", "")
                    )
                else:
                    path = f"{os.path.dirname(base_url[2])}/{new_url}"
                    new_url = urlunparse(
                        (base_url[0], base_url[1], path, "", "", "")
                    )

            return open_url(new_url)

        return utf8_reader(body, errors="strict")

    return open(url, mode="rt", encoding="utf-8")


class open_from_url:
    FORMAT_ERROR = "only asciicast v1 and v2 formats can be opened"

    def __init__(self, url: str) -> None:
        self.url = url
        self.file: Union[StreamReader, TextIO, None] = None
        self.context: Any = None

    def __enter__(self) -> Any:
        try:
            self.file = open_url(self.url)
            first_line = self.file.readline()

            try:  # try v2 first
                self.context = v2.open_from_file(first_line, self.file)
                return self.context.__enter__()
            except v2.LoadError:
                try:  # try v1 next
                    self.context = v1.open_from_file(first_line, self.file)
                    return self.context.__enter__()
                except v1.LoadError as e:
                    raise LoadError(self.FORMAT_ERROR) from e

        except (OSError, urllib.error.HTTPError) as e:
            raise LoadError(str(e)) from e

    def __exit__(
        self, exc_type: str, exc_value: str, exc_traceback: str
    ) -> None:
        self.context.__exit__(exc_type, exc_value, exc_traceback)
