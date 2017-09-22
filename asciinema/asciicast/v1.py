class Asciicast:

    def __init__(self, stdout):
        self.version = 1
        self.__stdout = stdout
        self.max_wait = None  # v1 doesn't store it

    def stdout(self):
        return self.__stdout


def load_from_dict(attrs):
    return Asciicast(attrs['stdout'])
