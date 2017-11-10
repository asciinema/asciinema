def to_relative_time(frames):
    prev_time = 0

    for frame in frames:
        time, data = frame
        delay = time - prev_time
        prev_time = time
        yield [delay, data]


def to_absolute_time(frames):
    time = 0

    for frame in frames:
        delay, data = frame
        time = time + delay
        yield [time, data]
