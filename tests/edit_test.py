import os

from nose.tools import assert_equal
from asciinema.commands import edit


class TestEdit:

    def setUp(self):
        self.events = []

    def write_event(self, ts, etype, data):
        self.events.append([ts, etype, data])

    def test_simple(self):
        input_events = [
            [0, "o", "foo"],
            [1, "d", "bar"],
        ]
        edit.edit_events(input_events, self.write_event)
        assert_equal([[0, "o", "foo"]], self.events)
