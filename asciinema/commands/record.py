import sys
import subprocess

from asciinema.recorder import Recorder
from asciinema.uploader import Uploader, ServerMaintenanceError, ResourceNotFoundError
from asciinema.confirmator import Confirmator


class RecordCommand(object):

    def __init__(self, api_url, api_token, cmd, title, skip_confirmation,
                 do_not_upload, recorder=None, uploader=None, confirmator=None):
        self.api_url = api_url
        self.api_token = api_token
        self.cmd = cmd
        self.title = title
        self.skip_confirmation = skip_confirmation
        self.do_not_upload = do_not_upload
        self.recorder = recorder if recorder is not None else Recorder()
        self.uploader = uploader if uploader is not None else Uploader()
        self.confirmator = confirmator if confirmator is not None else Confirmator()

    def execute(self):
        asciicast = self._record_asciicast()
        self._upload_asciicast(asciicast)

    def _record_asciicast(self):
        self._reset_terminal()
        print('~ Asciicast recording started.')

        if not self.cmd:
            print('~ Hit ctrl+d or type "exit" to finish.')

        print('')

        asciicast = self.recorder.record(self.cmd, self.title)

        self._reset_terminal()
        print('~ Asciicast recording finished.')

        return asciicast

    def _upload_asciicast(self, asciicast):
        if self._upload_confirmed():
            print('~ Uploading...')
            try:
                url = self.uploader.upload(self.api_url, self.api_token, asciicast)
                print(url)
            except ServerMaintenanceError:
                print('~ Upload failed: The server is down for maintenance. Try again in a minute.')
                sys.exit(1)
            except ResourceNotFoundError:
                print('~ Upload failed: Your client version is no longer supported. Please upgrade to the latest version.')
                sys.exit(1)

    def _upload_confirmed(self):
        if self.do_not_upload:
            return False
        if self.skip_confirmation:
            return True

        return self.confirmator.confirm("~ Do you want to upload it? [Y/n] ")

    def _reset_terminal(self):
        subprocess.call(["reset"])
        pass
