import sys
import subprocess

from nose.tools import assert_equal
from asciinema.commands.record import RecordCommand
from .test_helper import assert_printed, assert_not_printed, Test


class FakeAsciicast(object):

    def __init__(self, cmd, title):
        self.cmd = cmd
        self.title = title


class FakeRecorder(object):

    def __init__(self):
        self.asciicast = None

    def record(self, cmd, title):
        self.asciicast = FakeAsciicast(cmd, title)
        return self.asciicast


class FakeUploader(object):

    def __init__(self):
        self.uploaded = None

    def upload(self, api_url, user_token, asciicast):
        self.uploaded = [api_url, user_token, asciicast]
        return 'http://asciicast/url'


class FakeConfirmator(object):

    def __init__(self):
        self.text = ''
        self.success = True

    def confirm(self, text):
        self.text = text
        return self.success


class TestRecordCommand(Test):

    def setUp(self):
        Test.setUp(self)
        self.recorder = FakeRecorder()
        self.uploader = FakeUploader()
        self.confirmator = FakeConfirmator()
        self.real_subprocess_call = subprocess.call
        subprocess.call = lambda *args: None

    def tearDown(self):
        subprocess.call = self.real_subprocess_call

    def create_command(self, skip_confirmation):
        return RecordCommand('http://the/url', 'a1b2c3', 'ls -l', 'the title',
                             skip_confirmation, self.recorder, self.uploader,
                             self.confirmator)

    def test_execute_when_upload_confirmation_skipped(self):
        command = self.create_command(True)
        self.confirmator.success = False

        command.execute()

        assert 'Do you want to upload' not in self.confirmator.text
        self.assert_recorded_and_uploaded()

    def test_execute_when_upload_confirmed(self):
        command = self.create_command(False)
        self.confirmator.success = True

        command.execute()

        assert 'Do you want to upload' in self.confirmator.text
        self.assert_recorded_and_uploaded()

    def test_execute_when_upload_rejected(self):
        command = self.create_command(False)
        self.confirmator.success = False

        command.execute()

        assert 'Do you want to upload' in self.confirmator.text
        self.assert_recorded_but_not_uploaded()

    def assert_recorded_but_not_uploaded(self):
        asciicast = self.recorder.asciicast
        assert asciicast, 'asciicast not recorded'
        assert_not_printed('Uploading...')
        assert_equal(None, self.uploader.uploaded)

    def assert_recorded_and_uploaded(self):
        asciicast = self.recorder.asciicast
        assert asciicast, 'asciicast not recorded'
        assert_equal('ls -l', asciicast.cmd)
        assert_equal('the title', asciicast.title)
        assert_printed('Uploading...')
        assert_equal(['http://the/url', 'a1b2c3', asciicast], self.uploader.uploaded)
        assert_printed('http://asciicast/url')
