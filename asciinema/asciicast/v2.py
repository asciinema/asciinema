import json
import json.decoder
import time
import codecs

try:
    # Importing synchronize is to detect platforms where
    # multiprocessing does not work (python issue 3770)
    # and cause an ImportError. Otherwise it will happen
    # later when trying to use Queue().
    from multiprocessing import synchronize, Process, Queue
except ImportError:
    from threading import Thread as Process
    from queue import Queue


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


def build_header(metadata):
    header = {}
    header.update(metadata)
    header['version'] = 2

    assert 'width' in header, 'width missing in metadata'
    assert 'height' in header, 'height missing in metadata'
    assert type(header['width']) == int
    assert type(header['height']) == int

    if 'timestamp' in header:
        assert type(header['timestamp']) == int or type(header['timestamp']) == float

    return header


class writer():

    def __init__(self, path, width=None, height=None, header=None, mode='w', buffering=-1):
        self.path = path
        self.mode = mode
        self.buffering = buffering
        self.stdin_decoder = codecs.getincrementaldecoder('UTF-8')('replace')
        self.stdout_decoder = codecs.getincrementaldecoder('UTF-8')('replace')

        if mode == 'w':
            self.header = {'version': 2, 'width': width, 'height': height}
            self.header.update(header or {})
            assert type(self.header['width']) == int, 'width or header missing'
            assert type(self.header['height']) == int, 'height or header missing'
        else:
            self.header = None

    def __enter__(self):
        self.file = open(self.path, mode=self.mode, buffering=self.buffering)

        if self.header:
            self.__write_line(self.header)

        return self

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.file.close()

    def write_event(self, ts, etype=None, data=None):
        if etype is None:
            ts, etype, data = ts

        ts = round(ts, 6)

        if etype == 'o':
            if type(data) == str:
                data = data.encode(encoding='utf-8', errors='strict')
            text = self.stdout_decoder.decode(data)
            self.__write_line([ts, etype, text])
        elif etype == 'i':
            if type(data) == str:
                data = data.encode(encoding='utf-8', errors='strict')
            text = self.stdin_decoder.decode(data)
            self.__write_line([ts, etype, text])
        else:
            self.__write_line([ts, etype, data])

    def write_stdout(self, ts, data):
        self.write_event(ts, 'o', data)

    def write_stdin(self, ts, data):
        self.write_event(ts, 'i', data)

    def __write_line(self, obj):
        line = json.dumps(obj, ensure_ascii=False, indent=None, separators=(', ', ': '))
        self.file.write(line + '\n')


def write_json_lines_from_queue(path, header, mode, queue):
    with writer(path, header=header, mode=mode, buffering=1) as w:
        for event in iter(queue.get, None):
            w.write_event(event)


class async_writer():

    def __init__(self, path, metadata, append=False, time_offset=0):
        if append:
            assert time_offset > 0

        self.path = path
        self.metadata = metadata
        self.append = append
        self.time_offset = time_offset
        self.queue = Queue()

    def __enter__(self):
        header = build_header(self.metadata)
        mode = 'a' if self.append else 'w'
        self.process = Process(
            target=write_json_lines_from_queue,
            args=(self.path, header, mode, self.queue)
        )
        self.process.start()
        self.start_time = time.time() - self.time_offset
        return self

    def __exit__(self, exc_type, exc_value, exc_traceback):
        self.queue.put(None)
        self.process.join()

    def write_stdin(self, data):
        ts = time.time() - self.start_time
        self.queue.put([ts, 'i', data])

    def write_stdout(self, data):
        ts = time.time() - self.start_time
        self.queue.put([ts, 'o', data])
