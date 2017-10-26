from asciinema.commands.command import Command


class AuthCommand(Command):

    def __init__(self, api):
        Command.__init__(self)
        self.api = api

    def execute(self):
        self.print('Open the following URL in a browser to register your API token\n'
                   'and assign any recorded asciicasts to your %s profile.\n\n'
                   '%s\n' % (self.api.hostname(), self.api.auth_url()))
