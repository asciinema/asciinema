import time

from nose.tools import assert_equal
from .test_helper import Test, FakeClock
from asciinema.timer import timeit


class TestTimer(Test):

    def setUp(self):
        self.real_time = time.time
        time.time = FakeClock([10.0, 24.57]).time

    def tearDown(self):
        time.time = self.real_time

    def test_timeit(self):
        duration, return_value = timeit(lambda *args: args, 1, 'two', True)

        assert_equal(14.57, duration)
        assert_equal((1, 'two', True), return_value)
