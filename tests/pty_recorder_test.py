import os
import pty

from nose.tools import assert_equal
from .test_helper import Test

from asciinema.pty_recorder import PtyRecorder


class FakeStdout:

    def __init__(self):
        self.data = []
        self.closed = False

    def write(self, data):
        self.data.append(data)

    def close(self):
        self.close = True


class TestPtyRecorder(Test):

    def setUp(self):
        self.real_os_write = os.write
        os.write = self.os_write

    def tearDown(self):
        os.write = self.real_os_write

    def os_write(self, fd, data):
        if fd != pty.STDOUT_FILENO:
            self.real_os_write(fd, data)

    def test_record_command_writes_to_stdout(self):
        pty_recorder = PtyRecorder()
        output = FakeStdout()

        command = ['python3', '-c', "import sys; import time; sys.stdout.write(\'foo\'); sys.stdout.flush(); time.sleep(0.01); sys.stdout.write(\'bar\')"]
        pty_recorder.record_command(command, output)

        assert_equal([b'foo', b'bar'], output.data)
