class AuthCommand(object):

    def __init__(self, api_url, user_token):
        self.api_url = api_url
        self.user_token = user_token

    def execute(self):
        url = '%s/connect/%s' % (self.api_url, self.user_token)
        print 'Open following URL in your browser to authenticate and/or ' \
            'claim recorded asciicasts:\n%s' % url
