import sys
from codecs import StreamReader
from io import StringIO
from typing import Optional, TextIO, Union

stdout: Optional[Union[TextIO, StreamReader]] = None


class Test:
    def setUp(self) -> None:
        global stdout  # pylint: disable=global-statement
        self.real_stdout = sys.stdout
        sys.stdout = stdout = StringIO()

    def tearDown(self) -> None:
        sys.stdout = self.real_stdout
