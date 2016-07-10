import time

from nose.tools import assert_equal
from .test_helper import Test, FakeClock
from asciinema.stdout import Stdout


class TestStdout(Test):

    def setUp(self):
        Test.setUp(self)
        self.real_time = time.time
        time.time = FakeClock([1, 3, 10, 13, 17]).time

    def tearDown(self):
        time.time = self.real_time

    def test_write(self):
        stdout = Stdout()

        stdout.write(b'foo')
        stdout.write(b'barbaz')
        stdout.write('żó'.encode('utf-8') + bytes([0xc5]))
        stdout.write(bytes([0x82]) + 'ć'.encode('utf-8'))

        assert_equal([[2, 'foo'], [7, 'barbaz'], [3, 'żó'], [4, 'łć']], stdout.frames)

    def test_close(self):
        stdout = Stdout()

        stdout.write(b'foo')
        stdout.write(b'barbaz')
        stdout.close()

        assert_equal(12, stdout.duration)
