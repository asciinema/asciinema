import json
import json.decoder

from asciinema.asciicast.events import to_absolute_time


try:
    JSONDecodeError = json.decoder.JSONDecodeError
except AttributeError:
    JSONDecodeError = ValueError


class LoadError(Exception):
    pass


class Asciicast:

    def __init__(self, attrs):
        self.version = 1
        self.__attrs = attrs
        self.idle_time_limit = None  # v1 doesn't store it

    @property
    def v2_header(self):
        keys = ['width', 'height', 'duration', 'command', 'title', 'env']
        header = {k: v for k, v in self.__attrs.items() if k in keys and v is not None}
        return header

    def __stdout_events(self):
        for time, data in self.__attrs['stdout']:
            yield [time, 'o', data]

    def events(self):
        return self.stdout_events()

    def stdout_events(self):
        return to_absolute_time(self.__stdout_events())


class open_from_file():
    FORMAT_ERROR = "only asciicast v1 format can be opened"

    def __init__(self, first_line, file):
        self.first_line = first_line
        self.file = file

    def __enter__(self):
        try:
            attrs = json.loads(self.first_line + self.file.read())

            if attrs.get('version') == 1:
                return Asciicast(attrs)
            else:
                raise LoadError(self.FORMAT_ERROR)
        except JSONDecodeError as e:
            raise LoadError(self.FORMAT_ERROR)

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.file.close()
