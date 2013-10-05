from nose.tools import assert_equal

from test_helper import Test
from asciinema.recorder import Recorder
import asciinema.timer


class FakePtyRecorder(object):

    class Stdout(object):
        pass

    def __init__(self):
        self.stdout = self.Stdout()
        self.command = None

    def record_command(self, *args):
        self.call_args = args

        return self.stdout

    def record_call_args(self):
        return self.call_args


class TestRecorder(Test):

    def setUp(self):
        Test.setUp(self)
        self.pty_recorder = FakePtyRecorder()
        self.real_timeit = asciinema.timer.timeit
        asciinema.timer.timeit = lambda c, *args: (123.45, c(*args))

    def tearDown(self):
        asciinema.timer.timeit = self.real_timeit

    def test_record_when_title_and_command_given(self):
        recorder = Recorder(self.pty_recorder)

        asciicast = recorder.record('ls -l', 'the title')

        assert_equal('the title', asciicast.title)
        assert_equal('ls -l', asciicast.command)
        assert_equal(('ls -l',), self.pty_recorder.record_call_args())
        assert_equal(123.45, asciicast.duration)
        assert_equal(self.pty_recorder.stdout, asciicast.stdout)

    def test_record_when_no_title_nor_command_given(self):
        env = { 'SHELL': '/bin/blush' }
        recorder = Recorder(self.pty_recorder, env)

        asciicast = recorder.record(None, None)

        assert_equal(None, asciicast.title)
        assert_equal(None, asciicast.command)
        assert_equal(('/bin/blush',), self.pty_recorder.record_call_args())
        assert_equal(123.45, asciicast.duration)
        assert_equal(self.pty_recorder.stdout, asciicast.stdout)
