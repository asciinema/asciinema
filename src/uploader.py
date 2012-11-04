import os
import subprocess


class CurlFormData(object):

    def __init__(self, namespace=None):
        self.namespace = namespace
        self.files = {}

    def add_file(self, name, filename):
        if self.namespace:
            name = '%s[%s]' % (self.namespace, name)

        self.files[name] = '@' + filename

    def form_file_args(self):
        return ' '.join(['-F %s="%s"' % (k, v) for k, v in self.files.iteritems()])


class Uploader(object):
    '''Asciicast uploader.

    Uploads recorded script to website using HTTP based API.
    '''

    def __init__(self, api_url):
        self.upload_url = '%s/api/asciicasts' % api_url

    def upload(self, asciicast):
        form_data = CurlFormData('asciicast')

        form_data.add_file('meta', asciicast.metadata_filename)

        form_data.add_file('stdout', asciicast.stdout_data_filename)
        form_data.add_file('stdout_timing', asciicast.stdout_timing_filename)

        if os.path.exists(asciicast.stdin_data_filename):
            form_data.add_file('stdin', asciicast.stdin_data_filename)
            form_data.add_file('stdin_timing', asciicast.stdin_timing_filename)

        cmd = "curl -sSf -o - %s %s" % (form_data.form_file_args(), self.upload_url)

        process = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE)
        stdout, stderr = process.communicate()

        if stderr:
            # print >> sys.stderr, stderr
            # sys.stderr.write(stderr)
            os.write(2, stderr)

        return stdout
