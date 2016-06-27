import json
from .stdout import Stdout


class Asciicast(object):

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
        attrs = {
            "version": 1,
            "width": self.width,
            "height": self.height,
            "duration": self.duration,
            "command": self.command,
            "title": self.title,
            "env": {
                "TERM": self.term,
                "SHELL": self.shell
            },
            "stdout": self.stdout
        }

        with open(path, "w") as f:
            json_string = json.dumps(attrs, ensure_ascii=False, indent=2, default=self.json_default)
            f.write(json_string)

    def json_default(self, o):
        if isinstance(o, Stdout):
            return o.frames

        return json.JSONEncoder.default(self, o)
