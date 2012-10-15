import os
import subprocess
import shutil

class Uploader(object):
    '''Asciicast uploader.

    Uploads recorded script to website using HTTP based API.
    '''

    def __init__(self, api_url, path):
        self.api_url = api_url
        self.path = path

    def upload(self):
        print '~ Uploading...'

        files = {
                     'meta': 'meta.json',
                   'stdout': 'stdout',
            'stdout_timing': 'stdout.time'
        }

        if os.path.exists(self.path + '/stdin'):
            files['stdin']        = 'stdin'
            files['stdin_timing'] = 'stdin.time'

        fields = ["-F asciicast[%s]=@%s/%s" % (f, self.path, files[f]) for f in files]

        cmd = "curl -sSf -o - %s %s" % (' '.join(fields), '%s/api/asciicasts' % self.api_url)

        process = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE)
        stdout, stderr = process.communicate()

        if stderr:
            # print >> sys.stderr, stderr
            # sys.stderr.write(stderr)
            os.write(2, stderr)
        else:
            self._remove_files()

        if stdout:
            return stdout
        else:
            return None

    def _remove_files(self):
        shutil.rmtree(self.path)
