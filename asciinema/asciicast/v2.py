import json
import json.decoder
import time
import codecs

try:
    JSONDecodeError = json.decoder.JSONDecodeError
except AttributeError:
    JSONDecodeError = ValueError


class LoadError(Exception):
    pass


class Asciicast:

    def __init__(self, f, header):
        self.version = 2
        self.__file = f
        self.v2_header = header
        self.idle_time_limit = header.get('idle_time_limit')

    def events(self):
        for line in self.__file:
            yield json.loads(line)

    def stdout_events(self):
        for time, type, data in self.events():
            if type == 'o':
                yield [time, type, data]


def build_from_header_and_file(header, f):
    return Asciicast(f, header)


class open_from_file():
    FORMAT_ERROR = "only asciicast v2 format can be opened"

    def __init__(self, first_line, file):
        self.first_line = first_line
        self.file = file

    def __enter__(self):
        try:
            v2_header = json.loads(self.first_line)
            if v2_header.get('version') == 2:
                return build_from_header_and_file(v2_header, self.file)
            else:
                raise LoadError(self.FORMAT_ERROR)
        except JSONDecodeError as e:
            raise LoadError(self.FORMAT_ERROR)

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.file.close()


def get_duration(path):
    with open(path, mode='rt', encoding='utf-8') as f:
        first_line = f.readline()
        with open_from_file(first_line, f) as a:
            for last_frame in a.stdout_events():
                pass
            return last_frame[0]


def build_header(width, height, metadata):
    header = {'version': 2, 'width': width, 'height': height}
    header.update(metadata)

    assert 'width' in header, 'width missing in metadata'
    assert 'height' in header, 'height missing in metadata'
    assert type(header['width']) == int
    assert type(header['height']) == int

    if 'timestamp' in header:
        assert type(header['timestamp']) == int or type(header['timestamp']) == float

    return header


class writer():

    def __init__(self, path, metadata=None, append=False, buffering=1, width=None, height=None):
        self.path = path
        self.buffering = buffering
        self.stdin_decoder = codecs.getincrementaldecoder('UTF-8')('replace')
        self.stdout_decoder = codecs.getincrementaldecoder('UTF-8')('replace')

        if append:
            self.mode = 'a'
            self.header = None
        else:
            self.mode = 'w'
            self.header = build_header(width, height, metadata or {})

    def __enter__(self):
        self.file = open(self.path, mode=self.mode, buffering=self.buffering)

        if self.header:
            self.__write_line(self.header)

        return self

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.file.close()

    def write_stdout(self, ts, data):
        if type(data) == str:
            data = data.encode(encoding='utf-8', errors='strict')
        data = self.stdout_decoder.decode(data)
        self.__write_event(ts, 'o', data)

    def write_stdin(self, ts, data):
        if type(data) == str:
            data = data.encode(encoding='utf-8', errors='strict')
        data = self.stdin_decoder.decode(data)
        self.__write_event(ts, 'i', data)

    def __write_event(self, ts, etype, data):
        self.__write_line([round(ts, 6), etype, data])

    def __write_line(self, obj):
        line = json.dumps(obj, ensure_ascii=False, indent=None, separators=(', ', ': '))
        self.file.write(line + '\n')
