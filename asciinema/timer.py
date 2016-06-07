import time


def timeit(callable, *args):
    start_time = time.time()
    ret = callable(*args)
    end_time = time.time()
    duration = end_time - start_time

    return (duration, ret)
