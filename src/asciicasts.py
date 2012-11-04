import os
import glob
import shutil
import json

from timed_file import TimedFile


def pending(dir):
    filenames = glob.glob(dir + '/*/*.time')
    return [Asciicast(os.path.dirname(f)) for f in filenames]


class Asciicast(object):

    def __init__(self, dir_path):
        self.dir_path = dir_path

        if not os.path.isdir(self.dir_path):
            os.makedirs(self.dir_path)

        self.stdout_file = TimedFile(self.dir_path + '/stdout')
        self.stdin_file  = TimedFile(self.dir_path + '/stdin')

    @property
    def metadata_filename(self):
        return self.dir_path + '/meta.json'

    @property
    def stdout_data_filename(self):
        return self.stdout_file.data_filename

    @property
    def stdout_timing_filename(self):
        return self.stdout_file.timing_filename

    @property
    def stdin_data_filename(self):
        return self.stdin_file.data_filename

    @property
    def stdin_timing_filename(self):
        return self.stdin_file.timing_filename

    def open_files(self):
        self.stdout_file.start_timing()
        self.stdin_file.start_timing()

    def close_files(self):
        self.stdout_file.close()
        self.stdin_file.close()

    def save_metadata(self, data):
        json_string = json.dumps(data, sort_keys=True, indent=4)
        with open(self.metadata_filename, 'wb') as f:
            f.write(json_string + '\n')

    def destroy(self):
        shutil.rmtree(self.dir_path)
