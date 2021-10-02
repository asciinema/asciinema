import os
import pty

from nose.tools import assert_equal
from .test_helper import Test

import asciinema.pty


class FakeStdout:

    def __init__(self):
        self.data = []

    def write_stdout(self, ts, data):
        self.data.append(data)

    def write_stdin(self, ts, data):
        pass


class TestRecord(Test):

    def setUp(self):
        self.real_os_write = os.write
        os.write = self.os_write

    def tearDown(self):
        os.write = self.real_os_write

    def os_write(self, fd, data):
        if fd != pty.STDOUT_FILENO:
            self.real_os_write(fd, data)

    def test_record_command_writes_to_stdout(self):
        output = FakeStdout()

        command = ['python3', '-c', "import sys; import time; sys.stdout.write(\'foo\'); sys.stdout.flush(); time.sleep(0.01); sys.stdout.write(\'bar\')"]
        asciinema.pty.record(command, output)

        assert_equal([b'foo', b'bar'], output.data)
