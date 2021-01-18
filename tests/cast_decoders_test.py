import tempfile
import unittest
import gzip
import os

import asciinema.asciicast as asciicast


DIR = os.path.dirname(os.path.realpath(__file__))
DEMO_JSON_PATH = os.path.join(DIR, "demo.json")
DEMO_CAST_PATH = os.path.join(DIR, "demo.cast")
DEMO_JSON_GZ_PATH = os.path.join(DIR, "demo.json.gz")
DEMO_CAST_GZ_PATH = os.path.join(DIR, "demo.cast.gz")
DEMO_JSON_BZ2_PATH = os.path.join(DIR, "demo.json.bz2")
DEMO_CAST_BZ2_PATH = os.path.join(DIR, "demo.cast.bz2")


def test_decodes_json_file():
    with asciicast.open_from_url(DEMO_JSON_PATH) as a:
        pass


def test_decodes_cast_file():
    with asciicast.open_from_url(DEMO_CAST_PATH) as a:
        pass


def test_decodes_json_gz_file():
    with asciicast.open_from_url(DEMO_JSON_GZ_PATH) as a, asciicast.open_from_url(DEMO_JSON_PATH) as b:
        assert a.v2_header == b.v2_header
        assert list(a.events()) == list(b.events())


def test_decodes_cast_gz_file():
    with asciicast.open_from_url(DEMO_CAST_GZ_PATH) as a, asciicast.open_from_url(DEMO_CAST_PATH) as b:
        assert a.v2_header == b.v2_header
        assert list(a.events()) == list(b.events())


def test_decodes_json_bz2_file():
    with asciicast.open_from_url(DEMO_JSON_BZ2_PATH) as a, asciicast.open_from_url(DEMO_JSON_PATH) as b:
        assert a.v2_header == b.v2_header
        assert list(a.events()) == list(b.events())


def test_decodes_cast_bz2_file():
    with asciicast.open_from_url(DEMO_CAST_BZ2_PATH) as a, asciicast.open_from_url(DEMO_CAST_PATH) as b:
        assert a.v2_header == b.v2_header
        assert list(a.events()) == list(b.events())
