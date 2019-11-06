import codecs
import gzip
import zipfile


def gzip_encode(input, errors='strict'):
    assert errors == 'strict'
    return (gzip.compress(input), len(input))


def gzip_decode(input, errors='strict'):
    assert errors == 'strict'
    return (gzip.decompress(input), len(input))


class GzipCodec(codecs.Codec):
    def encode(self, input, errors='strict'):
        return gzip_encode(input, errors)

    def decode(self, input, errors='strict'):
        return gzip_decode(input, errors)


class GzipStreamWriter(GzipCodec, codecs.StreamWriter):
    charbuffertype = bytes


class GzipStreamReader(GzipCodec, codecs.StreamReader):
    charbuffertype = bytes


gzip_codec = codecs.CodecInfo(
    name="gzip",
    encode=gzip_encode,
    decode=gzip_decode,
    streamreader=GzipStreamReader,
    streamwriter=GzipStreamWriter,
    _is_text_encoding=False,
)


def search_function(encoding):
    if encoding == "gzip":
        return gzip_codec
    return None


codecs.register(search_function)


def setup():
    codecs.register(search_function)
