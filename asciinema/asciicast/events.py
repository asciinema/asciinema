def to_relative_time(events):
    prev_time = 0

    for frame in events:
        time, type, data = frame
        delay = time - prev_time
        prev_time = time
        yield [delay, type, data]


def to_absolute_time(events):
    time = 0

    for frame in events:
        delay, type, data = frame
        time = time + delay
        yield [time, type, data]


def cap_relative_time(events, time_limit):
    if time_limit:
        return ([min(delay, time_limit), type, data] for delay, type, data in events)
    else:
        return events


def adjust_speed(events, speed):
    return ([delay / speed, type, data] for delay, type, data in events)
