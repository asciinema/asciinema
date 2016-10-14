from asciinema.commands.command import Command


class AuthCommand(Command):

    def __init__(self, api_url, api_token):
        Command.__init__(self)
        self.api_url = api_url
        self.api_token = api_token

    def execute(self):
        url = '%s/connect/%s' % (self.api_url, self.api_token)
        self.print('Open the following URL in a browser to register your API '
                   'token and assign any recorded asciicasts to your profile:\n'
                   '%s' % url)
