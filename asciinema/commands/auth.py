from asciinema.commands.command import Command


class AuthCommand(Command):
    """
    Handles authentication using install ID

    Attributes:
        api (asciinema.API): api that will be used for authentication
    """
    def __init__(self, api):
        Command.__init__(self)
        self.api = api

    def execute(self):
        """
        Sends you to the site and allows you to authenticate.
        """
        self.print('Open the following URL in a web browser to link your '
                   'install ID with your %s user account:\n\n'
                   '%s\n\n'
                   'This will associate all recordings uploaded from this machine '
                   '(past and future ones) to your account, '
                   'and allow you to manage them (change title/theme, delete) at %s.'
                   % (self.api.hostname(), self.api.auth_url(), self.api.hostname()))
