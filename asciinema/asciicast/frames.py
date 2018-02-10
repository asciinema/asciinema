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


def cap_relative_time(frames, time_limit):
    if time_limit:
        return ([min(delay, time_limit), text] for delay, text in frames)
    else:
        return frames


def adjust_speed(frames, speed):
    return ([delay / speed, text] for delay, text in frames)
