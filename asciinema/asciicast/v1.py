import json
import json.decoder

from asciinema.asciicast.frames import to_absolute_time


try:
    JSONDecodeError = json.decoder.JSONDecodeError
except AttributeError:
    JSONDecodeError = ValueError


class LoadError(Exception):
    pass


class Asciicast:

    def __init__(self, stdout):
        self.version = 1
        self.__stdout = stdout
        self.idle_time_limit = None  # v1 doesn't store it

    def stdout(self):
        return to_absolute_time(self.__stdout)


class open_from_file():
    FORMAT_ERROR = "only asciicast v1 format can be opened"

    def __init__(self, first_line, file):
        self.first_line = first_line
        self.file = file

    def __enter__(self):
        try:
            attrs = json.loads(self.first_line + self.file.read())

            if attrs.get('version') == 1:
                return Asciicast(attrs['stdout'])
            else:
                raise LoadError(self.FORMAT_ERROR)
        except JSONDecodeError as e:
            raise LoadError(self.FORMAT_ERROR)

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.file.close()
