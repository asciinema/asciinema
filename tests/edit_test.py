import os

from nose.tools import assert_equal
from asciinema.commands import edit


class TestEdit:

    def setUp(self):
        self.events = []

    def write_event(self, ts, etype, data):
        self.events.append([ts, etype, data])

    def test_delete(self):
        input_events = [
            [0, "o", "foo"],
            [1, "d", "bar"],
        ]
        edit.edit_events(input_events, self.write_event)
        assert_equal([[0, "o", "foo"]], self.events)

    def test_squash(self):
        input_events = [
            [0, "o", "foo"],
            [1, "s", "bar"],
            [2, "s", "baz"],
        ]
        edit.edit_events(input_events, self.write_event)
        assert_equal([[0, "o", "foobarbaz"]], self.events)

    def test_delete_and_squash(self):
        input_events = [
            [0, "o", "foo"],
            [1, "d", "XXX"],
            [2, "s", "bar"],
            [3, "o", "next"],
            [4, "s", " baz"]
        ]
        expected = [
            [0, "o", "foobar"],
            [3, "o", "next baz"],
        ]
        edit.edit_events(input_events, self.write_event)
        assert_equal(expected, self.events)


