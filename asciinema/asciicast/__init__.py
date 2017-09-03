import sys
import json.decoder
import urllib.request
import urllib.error
import html.parser
import tempfile
import shutil
import io

from . import v1
from . import v2


try:
    JSONDecodeError = json.decoder.JSONDecodeError
except AttributeError:
    JSONDecodeError = ValueError


class LoadError(Exception):
    pass


class Parser(html.parser.HTMLParser):
    def __init__(self):
        html.parser.HTMLParser.__init__(self)
        self.url = None

    def handle_starttag(self, tag, attrs_list):
        # look for <link rel="alternate" type="application/asciicast+json" href="https://...json">
        if tag == 'link':
            attrs = {}
            for k, v in attrs_list:
                attrs[k] = v

            if attrs.get('rel') == 'alternate' and attrs.get('type') == 'application/asciicast+json':
                self.url = attrs.get('href')


def download_url(url):
    if url.startswith("ipfs:/"):
        url = "https://ipfs.io/%s" % url[6:]
    elif url.startswith("fs:/"):
        url = "https://ipfs.io/%s" % url[4:]

    if url == "-":
        tmp_file = tempfile.SpooledTemporaryFile(max_size=10000000, mode='w+')
        shutil.copyfileobj(sys.stdin, tmp_file)
        tmp_file.seek(0)
        return tmp_file

    if url.startswith("http:") or url.startswith("https:"):
        response = urllib.request.urlopen(url)
        data = response.read().decode(errors='replace')

        content_type = response.headers['Content-Type']
        if content_type and content_type.startswith('text/html'):
            parser = Parser()
            parser.feed(data)
            url = parser.url

            if not url:
                raise LoadError("""<link rel="alternate" type="application/asciicast+json" href="..."> not found in fetched HTML document""")

            return download_url(url)

        return io.StringIO(data)

    return open(url, 'r')


class open_from_url():
    FORMAT_ERROR = "only asciicast v1 and v2 formats can be opened"

    def __init__(self, url):
        self.url = url

    def __enter__(self):
        try:
            self.file = download_url(self.url)
            line = self.file.readline()
            self.file.seek(0)

            try:  # parse it as v2
                v2_header = json.loads(line)
                if v2_header.get('version') == 2:
                    return v2.load_from_file(self.file)
                else:
                    raise LoadError(self.FORMAT_ERROR)
            except JSONDecodeError as e:
                try:  # parse it as v1
                    attrs = json.load(self.file)
                    self.file.close()
                    if attrs.get('version') == 1:
                        return v1.load_from_dict(attrs)
                    else:
                        raise LoadError(self.FORMAT_ERROR)
                except JSONDecodeError as e:
                    raise LoadError(self.FORMAT_ERROR)
        except (OSError, urllib.error.HTTPError) as e:
            raise LoadError(str(e))

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.file.close()
