#!/usr/bin/env python3
"""Validate engine-metrics JSONL and summarize process metrics, not gameplay."""
import argparse
import json
import math
from pathlib import Path
import statistics


def number(value, name, minimum=0):
    if type(value) not in (int, float) or not math.isfinite(value) or value < minimum:
        raise ValueError(f'Invalid {name}')
    return value


def integer(value, name, minimum=1):
    if type(value) is not int or value < minimum:
        raise ValueError(f'Invalid {name}')
    return value


def summarize(rows, min_duration=0):
    number(min_duration, 'minimum duration')
    if len(rows) < 3 or any(not isinstance(row, dict) for row in rows):
        raise ValueError('Missing start/sample/end records')
    start, end = rows[0], rows[-1]
    if start.get('event') != 'start' or type(start.get('schema')) is not int or start['schema'] != 1:
        raise ValueError('Unsupported metrics schema')
    if start.get('rss_unit') != 'KiB':
        raise ValueError('Unexpected RSS unit')
    pid = integer(start.get('pid'), 'PID')
    count = integer(start.get('samples_requested'), 'requested sample count')
    interval = number(start.get('interval_seconds'), 'interval', 1)
    if end.get('event') != 'end' or end.get('complete') is not True:
        raise ValueError('Collection did not complete')
    samples = rows[1:-1]
    if integer(end.get('samples_written'), 'written count') != count or len(samples) != count:
        raise ValueError('Missing or extra samples')
    elapsed, rss, cpu = [], [], []
    for index, sample in enumerate(samples):
        if sample.get('event') != 'sample' or integer(sample.get('index'), 'index', 0) != index:
            raise ValueError('Out-of-order sample sequence')
        elapsed.append(number(sample.get('elapsed_seconds'), 'elapsed time'))
        number(sample.get('unix_seconds'), 'wall clock', 1)
        rss.append(integer(sample.get('rss_kib'), 'RSS'))
        cpu.append(number(sample.get('cpu_percent_ps'), 'CPU'))
    gaps = [later - earlier for earlier, later in zip(elapsed, elapsed[1:])]
    if any(gap <= 0 for gap in gaps):
        raise ValueError('Non-monotonic sampling clock')
    if any(gap > interval * 2 for gap in gaps):
        raise ValueError('Sampling gap exceeds twice the requested interval')
    duration = elapsed[-1] - elapsed[0]
    if duration < min_duration:
        raise ValueError(f'Insufficient measured duration: {duration:.3f}s < {min_duration:.3f}s')
    return {
        'schema': 1, 'collection_valid': True, 'pid': pid, 'samples': count,
        'measured_span_seconds': duration, 'maximum_gap_seconds': max(gaps, default=0),
        'rss_kib': {'sample_mean': statistics.mean(rss), 'peak_sample': max(rss)},
        'cpu_percent_ps': {'sample_mean': statistics.mean(cpu), 'peak_sample': max(cpu)},
        'limitations': ['One process only; excludes helpers/GPU allocations',
                        'ps CPU estimate, not interval CPU utilization',
                        'Arithmetic sample means, not time-weighted means',
                        'No gameplay, FPS, audio, memory pressure or engine acceptance conclusion'],
    }


def load(path):
    if path.stat().st_size > 16 * 1024 * 1024:
        raise ValueError('Metrics file exceeds 16 MiB limit')
    rows = []
    with path.open() as handle:
        for line in handle:
            if not line.strip():
                raise ValueError('Blank metrics record')
            rows.append(json.loads(line))
            if len(rows) > 10002:
                raise ValueError('Too many metrics records')
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input', type=Path)
    parser.add_argument('--min-duration', type=float, default=0, help='Required span between first and last samples, in seconds')
    args = parser.parse_args()
    try:
        result = summarize(load(args.input), args.min_duration)
    except (OSError, ValueError, TypeError, KeyError) as error:
        parser.exit(1, f'Metrics rejected: {error}\n')
    print(json.dumps(result, indent=2, allow_nan=False))


if __name__ == '__main__':
    main()
