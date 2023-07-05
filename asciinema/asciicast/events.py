from typing import Any, Generator, Iterable, List, Optional


def to_relative_time(
    events: Iterable[Any],
) -> Generator[List[Any], None, None]:
    prev_time = 0

    for frame in events:
        time, type_, data = frame
        delay = time - prev_time
        prev_time = time
        yield [delay, type_, data]


def to_absolute_time(
    events: Iterable[Any],
) -> Generator[List[Any], None, None]:
    time = 0

    for frame in events:
        delay, type_, data = frame
        time = time + delay
        yield [time, type_, data]


def cap_relative_time(
    events: Iterable[Any], time_limit: Optional[float]
) -> Iterable[Any]:
    if time_limit:
        return (
            [min(delay, time_limit), type_, data]
            for delay, type_, data in events
        )
    return events


def adjust_speed(
    events: Iterable[Any], speed: Any
) -> Generator[List[Any], None, None]:
    return ([delay / speed, type_, data] for delay, type_, data in events)
