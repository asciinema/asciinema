import sys
import os
from urllib.request import Request, urlopen
from urllib.parse import urlparse, urlunparse
import urllib.error
import html.parser
import gzip
import codecs

from . import v1
from . import v2


class LoadError(Exception):
    pass


class Parser(html.parser.HTMLParser):
    def __init__(self):
        html.parser.HTMLParser.__init__(self)
        self.url = None

    def handle_starttag(self, tag, attrs_list):
        # look for <link rel="alternate" type="application/x-asciicast" href="https://...cast">
        if tag == 'link':
            attrs = {}
            for k, v in attrs_list:
                attrs[k] = v

            if attrs.get('rel') == 'alternate':
                type = attrs.get('type')
                if type == 'application/asciicast+json' or type == 'application/x-asciicast':
                    self.url = attrs.get('href')


def open_url(url):
    if url == "-":
        return sys.stdin

    if url.startswith("ipfs://"):
        url = "https://ipfs.io/ipfs/%s" % url[7:]
    elif url.startswith("dweb:/ipfs/"):
        url = "https://ipfs.io/%s" % url[5:]

    if url.startswith("http:") or url.startswith("https:"):
        req = Request(url)
        req.add_header('Accept-Encoding', 'gzip')
        response = urlopen(req)
        body = response
        url = response.geturl()  # final URL after redirects

        if response.headers['Content-Encoding'] == 'gzip':
            body = gzip.open(body)

        utf8_reader = codecs.getreader('utf-8')
        content_type = response.headers['Content-Type']

        if content_type and content_type.startswith('text/html'):
            html = utf8_reader(body, errors='replace').read()
            parser = Parser()
            parser.feed(html)
            new_url = parser.url

            if not new_url:
                raise LoadError("""<link rel="alternate" type="application/x-asciicast" href="..."> not found in fetched HTML document""")

            if "://" not in new_url:
                base_url = urlparse(url)

                if new_url.startswith("/"):
                    new_url = urlunparse((base_url[0], base_url[1], new_url, '', '', ''))
                else:
                    path = os.path.dirname(base_url[2]) + '/' + new_url
                    new_url = urlunparse((base_url[0], base_url[1], path, '', '', ''))

            return open_url(new_url)

        return utf8_reader(body, errors='strict')

    return open(url, mode='rt', encoding='utf-8')


class open_from_url():
    FORMAT_ERROR = "only asciicast v1 and v2 formats can be opened"

    def __init__(self, url):
        self.url = url

    def __enter__(self):
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
                except v1.LoadError:
                    raise LoadError(self.FORMAT_ERROR)

        except (OSError, urllib.error.HTTPError) as e:
            raise LoadError(str(e))

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.context.__exit__(exc_type, exc_value, exc_traceback)
