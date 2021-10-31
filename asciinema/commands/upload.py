from asciinema.api import APIError
from asciinema.commands.command import Command


class UploadCommand(Command):
    def __init__(self, args, config, env):
        Command.__init__(self, args, config, env)
        self.filename = args.filename

    def execute(self):
        try:
            result, warn = self.api.upload_asciicast(self.filename)

            if warn:
                self.print_warning(warn)

            self.print(result.get("message") or result["url"])

        except OSError as e:
            self.print_error(f"upload failed: {str(e)}")
            return 1

        except APIError as e:
            self.print_error(f"upload failed: {str(e)}")
            self.print_error(
                f"retry later by running: asciinema upload {self.filename}"
            )
            return 1

        return 0
