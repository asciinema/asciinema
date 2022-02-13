from typing import Any, Generator, List, Optional


def to_relative_time(
    events: Generator[List[Any], None, None]
) -> Generator[List[Any], None, None]:
    prev_time = 0

    for frame in events:
        time, type_, data = frame
        delay = time - prev_time
        prev_time = time
        yield [delay, type_, data]


def to_absolute_time(
    events: Generator[List[Any], None, None]
) -> Generator[List[Any], None, None]:
    time = 0

    for frame in events:
        delay, type_, data = frame
        time = time + delay
        yield [time, type_, data]


def cap_relative_time(
    events: Generator[List[Any], None, None], time_limit: Optional[float]
) -> Generator[List[Any], None, None]:
    if time_limit:
        return (
            [min(delay, time_limit), type_, data]
            for delay, type_, data in events
        )
    return events


def adjust_speed(
    events: Generator[List[Any], None, None], speed: Any
) -> Generator[List[Any], None, None]:
    return ([delay / speed, type_, data] for delay, type_, data in events)
