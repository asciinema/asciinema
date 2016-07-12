import sys
import json
import json.decoder
import urllib.request
import urllib.error
import html.parser
from .stdout import Stdout


class Asciicast:

    def __init__(self, stdout, width, height, duration, command=None, title=None, term=None, shell=None):
        self.stdout = stdout
        self.width = width
        self.height = height
        self.duration = duration
        self.command = command
        self.title = title
        self.term = term
        self.shell = shell

    def save(self, path):
        stdout = list(map(lambda frame: [round(frame[0], 6), frame[1]], self.stdout.frames))
        duration = round(self.duration, 6)
        attrs = {
            "version": 1,
            "width": self.width,
            "height": self.height,
            "duration": duration,
            "command": self.command,
            "title": self.title,
            "env": {
                "TERM": self.term,
                "SHELL": self.shell
            },
            "stdout": stdout
        }

        with open(path, "w") as f:
            f.write(json.dumps(attrs, ensure_ascii=False, indent=2))


# asciinema play file.json
# asciinema play https://asciinema.org/a/123.json
# asciinema play https://asciinema.org/a/123
# asciinema play ipfs://ipfs/QmbdpNCwqeZgnmAWBCQcs8u6Ts6P2ku97tfKAycE1XY88p
# asciinema play -


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


def fetch(url):
    if url.startswith("ipfs:/"):
        url = "https://ipfs.io/%s" % url[6:]
    elif url.startswith("fs:/"):
        url = "https://ipfs.io/%s" % url[4:]

    if url == "-":
        return sys.stdin.read()

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

            return fetch(url)

        return data

    with open(url, 'r') as f:
        return f.read()


def load(filename):
    try:
        attrs = json.loads(fetch(filename))

        if type(attrs) != dict:
            raise LoadError('unsupported asciicast format')

        return Asciicast(
            attrs['stdout'],
            attrs['width'],
            attrs['height'],
            attrs['duration'],
            attrs['command'],
            attrs['title']
        )
    except (OSError, urllib.error.HTTPError) as e:
        raise LoadError(str(e))
    except json.decoder.JSONDecodeError as e:
        raise LoadError('JSON decoding error: ' + str(e))
    except KeyError as e:
        raise LoadError('asciicast is missing key ' + str(e))
