import os
import sys
import glob

import help_text
from config import Config
from options import Options
from constants import SCRIPT_NAME
from asciicast import AsciiCast
from uploader import Uploader

class CLI:
    '''Parses command-line options and xxxxxxxxxxxxxxxxx.'''

    def run(self):
        self.config = Config()
        self.options = Options(sys.argv)

        action = self.options.action

        if action == 'rec':
            self.check_pending()
            self.record()
        elif action == 'upload':
            self.upload_pending()
        elif action == 'auth':
            self.auth()
        elif action == 'help':
            self.help()
        elif action == 'version':
            self.version()
        else:
            print('Unknown action: %s' % action)
            print('Run "%s --help" for list of available options' % sys.argv[0])

    def record(self):
        if not AsciiCast(self.config, self.options).create():
            sys.exit(1)

    def auth(self):
        url = '%s/connect/%s' % (self.config.api_url(), self.config.user_token())
        print 'Open following URL in your browser to authenticate and/or claim ' \
            'recorded asciicasts:\n\n%s' % url

    def check_pending(self):
        num = len(self.pending_list())
        if num > 0:
            print 'Warning: %i recorded asciicasts weren\'t uploaded. ' \
                'Run "%s upload" to upload them or delete them with "rm -rf %s/*".' \
                % (num, SCRIPT_NAME, AsciiCast.QUEUE_DIR)

    def upload_pending(self):
        print 'Uploading pending asciicasts...'
        for path in self.pending_list():
            url = Uploader(self.config, path).upload()
            if url:
                print url

    def pending_list(self):
        return [os.path.dirname(p) for p in glob.glob(AsciiCast.QUEUE_DIR + '/*/*.time')]

    def version(self):
        print 'asciiio 1.0.1'

    def help(self):
        print help_text.TEXT
