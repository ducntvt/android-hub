#!/usr/bin/env python3
"""Read-only macOS CPU/RSS sampler for explicitly selected display processes.

Does not launch viewers, discover phones, capture pixels, or measure input latency.
CPU is ps lifetime-weighted %cpu (100% = one core); RSS excludes GPU allocations.
"""
import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import time


def read_process(pid):
    result = subprocess.run(
        ['/bin/ps', '-p', str(pid), '-o', 'pid=,lstart=,%cpu=,rss=,comm='],
        capture_output=True, text=True, timeout=5,
        env={**os.environ, 'LC_ALL': 'C'},
    )
    if result.returncode == 1 and not result.stdout.strip():
        raise ValueError(f'Process {pid} exited')
    if result.returncode:
        raise ValueError(f'Cannot inspect process {pid}')
    return parse_process(result.stdout, pid)


def parse_process(line, expected_pid):
    fields = line.strip().split(None, 8)
    if len(fields) != 9 or int(fields[0]) != expected_pid:
        raise ValueError('Unexpected ps response')
    cpu, rss = float(fields[6]), int(fields[7])
    if not math.isfinite(cpu) or cpu < 0 or rss < 0:
        raise ValueError('Invalid process metrics')
    return {
        'pid': expected_pid, 'started': ' '.join(fields[1:6]),
        'executable': fields[8], 'cpu_percent': cpu, 'rss_kib': rss,
    }


def identity(process):
    return process['pid'], process['started'], process['executable']


def collect(pids, seconds, output, read=read_process, clock=time.monotonic, sleep=time.sleep):
    if not pids or len(set(pids)) != len(pids) or any(p <= 0 for p in pids):
        raise ValueError('Choose distinct positive process IDs')
    if not 1 <= seconds <= 120:
        raise ValueError('Duration must be 1–120 seconds')
    initial = [read(pid) for pid in pids]
    identities = [identity(p) for p in initial]
    # Exclusive creation: never overwrite evidence from a prior run.
    with Path(output).open('x', encoding='utf-8') as stream:
        def write(record):
            stream.write(json.dumps(record, ensure_ascii=False) + '\n')
            stream.flush()
        write({'type': 'metadata', 'schema': 1, 'duration_seconds': seconds,
               'processes': initial, 'interval_seconds': 1,
               'limitations': 'ps CPU is averaged; RSS excludes GPU; not FPS or input latency'})
        started = clock()
        deadline = started + seconds
        count = 0
        try:
            while clock() < deadline:
                sample = [read(pid) for pid in pids]
                if [identity(p) for p in sample] != identities:
                    raise ValueError('Process identity changed; measurement stopped')
                write({'type': 'sample', 'elapsed_seconds': clock() - started,
                       'processes': sample,
                       'cpu_percent_sum': sum(p['cpu_percent'] for p in sample),
                       'rss_kib_sum': sum(p['rss_kib'] for p in sample)})
                count += 1
                sleep(max(0, min(started + count, deadline) - clock()))
        except (ValueError, OSError, subprocess.SubprocessError, KeyboardInterrupt) as error:
            write({'type': 'end', 'complete': False, 'samples': count,
                   'reason': str(error) or 'Interrupted'})
            raise
        write({'type': 'end', 'complete': True, 'samples': count,
               'elapsed_seconds': clock() - started})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pid', type=int, action='append', required=True,
                        help='Explicit viewer/helper PID; repeat for additional processes')
    parser.add_argument('--seconds', type=int, default=120)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    try:
        collect(args.pid, args.seconds, args.output)
    except (ValueError, OSError, subprocess.SubprocessError, KeyboardInterrupt) as error:
        parser.exit(1, f'Measurement incomplete: {error}\n')


if __name__ == '__main__':
    main()
