class AuthCommand(object):

    def __init__(self, api_url, api_token):
        self.api_url = api_url
        self.api_token = api_token

    def execute(self):
        url = '%s/connect/%s' % (self.api_url, self.api_token)
        print('Open the following URL in your browser to register your API ' \
                'token and assign any recorded asciicasts to your profile:\n' \
                '%s' % url)
