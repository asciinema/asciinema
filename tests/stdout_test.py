import time

from nose.tools import assert_equal, assert_raises
from .test_helper import Test, FakeClock
from asciinema.stdout import Stdout, StdoutTiming


class TestStdoutTiming(Test):

    def test_append(self):
        timing = StdoutTiming()

        timing.append([0.123, 100])
        timing.append([1234.56, 33])

        assert_equal('0.123000 100\n1234.560000 33', str(timing))


class TestStdout(Test):

    def setUp(self):
        Test.setUp(self)
        self.real_time = time.time
        time.time = FakeClock([1, 3, 10]).time

    def tearDown(self):
        time.time = self.real_time

    def test_write(self):
        timing = []
        stdout = Stdout(timing)

        stdout.write(b'foo')
        stdout.write(b'barbaz')

        assert_equal(b'foobarbaz', stdout.data)
        assert_equal([[2, 3], [7, 6]], timing)

    def test_close(self):
        stdout = Stdout()

        stdout.close()

        assert_raises(ValueError, stdout.write, 'qux')
