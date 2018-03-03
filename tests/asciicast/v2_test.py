from ..test_helper import Test
import asciinema.asciicast.v2 as v2
import tempfile


class TestWriter(Test):

    def test_writing(self):
        _file, path = tempfile.mkstemp()

        with v2.writer(path, width=80, height=24) as w:
            w.write_stdout(1, 'x')  # ensure it supports both str and bytes
            w.write_stdout(2, bytes.fromhex('78 c5 bc c3 b3 c5'))
            w.write_stdout(3, bytes.fromhex('82 c4 87'))
            w.write_stdout(4, bytes.fromhex('78 78'))

        with open(path, 'r') as f:
            text = f.read()
            assert text == '{"version": 2, "width": 80, "height": 24}\n' + \
                           '[1, "o", "x"]\n' + \
                           '[2, "o", "xżó"]\n' + \
                           '[3, "o", "łć"]\n' + \
                           '[4, "o", "xx"]\n', 'got:\n\n%s' % text
