from asciinema.asciicast.frames import to_absolute_time


class Asciicast:

    def __init__(self, stdout):
        self.version = 1
        self.__stdout = stdout
        self.idle_time_limit = None  # v1 doesn't store it

    def stdout(self):
        return to_absolute_time(self.__stdout)


def load_from_dict(attrs):
    return Asciicast(attrs['stdout'])
