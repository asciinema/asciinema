from asciinema.commands.command import Command
from asciinema.api import APIError


class UploadCommand(Command):

    def __init__(self, args, config, env):
        Command.__init__(self, args, config, env)
        self.filename = args.filename

    def execute(self):
        try:
            result, warn = self.api.upload_asciicast(self.filename)

            if warn:
                self.print_warning(warn)

            self.print(result.get('message') or result['url'])

        except OSError as e:
            self.print_error("upload failed: %s" % str(e))
            return 1

        except APIError as e:
            self.print_error("upload failed: %s" % str(e))
            self.print_error("retry later by running: asciinema upload %s" % self.filename)
            return 1

        return 0
