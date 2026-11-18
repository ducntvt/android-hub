#!/usr/bin/env python3
"""Collect concurrent read-only engine traces; never starts/stops Android guests."""
import argparse
import importlib.util
import json
import math
import os
import signal
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('hub_metrics', ROOT / 'scripts/p0-metrics-summary.py')
metrics = importlib.util.module_from_spec(spec)
spec.loader.exec_module(metrics)


def settings(pids, duration, interval):
    if len(pids) not in (1, 2) or len(set(pids)) != len(pids) or any(pid <= 0 for pid in pids):
        raise ValueError('Provide one or two distinct positive engine PIDs')
    if not 60 <= duration <= 14400 or not 1 <= interval <= 5:
        raise ValueError('Duration must be 60..14400 seconds; interval 1..5 seconds')
    count = math.ceil(duration / interval) + 2
    if count > 10000:
        raise ValueError('Too many samples; increase interval')
    return count


def save(path, value):
    temporary = path.with_suffix('.json.new')
    with temporary.open('x') as handle:
        json.dump(value, handle, indent=2, allow_nan=False)
        handle.write('\n')
        handle.flush()
        os.fsync(handle.fileno())
    temporary.replace(path)


def collect(binary, pids, duration, interval, output, workload, configuration):
    count = settings(pids, duration, interval)
    if not workload.strip() or not configuration.strip():
        raise ValueError('Describe the actual workload and configuration')
    binary = binary.resolve(strict=True)
    output.mkdir(mode=0o700)  # Never overwrite or resume an old evidence directory.
    record = {'schema': 1, 'state': 'running', 'started_at_unix': time.time(),
              'duration_requested': duration, 'interval_seconds': interval,
              'workload': workload, 'configuration': configuration,
              'process_traces': [f'engine-{pid}.jsonl' for pid in pids],
              'engine_pids': pids, 'sampler_pids': [],
              'limitations': 'Process RSS/ps CPU only; no FPS, gameplay, audio or acceptance claim'}
    jobs, handles = [], []
    save(output / 'collection.json', record)
    try:
        for pid, trace in zip(pids, record['process_traces']):
            log = (output / f'sampler-{pid}.log').open('x')
            handles.append(log)
            job = subprocess.Popen([str(binary), str(pid), str(count), str(interval), str(output / trace)],
                                   stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
            jobs.append(job)
            record['sampler_pids'].append(job.pid)
            save(output / 'collection.json', record)
        deadline = time.monotonic() + (count - 1) * interval + 30
        while True:
            codes = [job.poll() for job in jobs]
            if any(code is not None and code != 0 for code in codes):
                raise RuntimeError('A sampler failed; inspect sampler logs')
            if all(code == 0 for code in codes):
                break
            if time.monotonic() >= deadline:
                raise TimeoutError('Sampler exceeded collection deadline')
            time.sleep(0.25)
        rows = [metrics.load(output / name) for name in record['process_traces']]
        summaries = [metrics.summarize(trace, duration) for trace in rows]
        if [summary['pid'] for summary in summaries] != pids:
            raise ValueError('Sampler output PID mismatch')
        for trace, summary in zip(rows, summaries):
            times = [row['unix_seconds'] for row in trace[1:-1]]
            if any(b <= a for a, b in zip(times, times[1:])) or abs(times[-1] - times[0] - summary['measured_span_seconds']) > 2:
                raise ValueError('Wall clock is non-monotonic or disagrees with elapsed time')
        overlap = min(trace[-2]['unix_seconds'] for trace in rows) - max(trace[1]['unix_seconds'] for trace in rows)
        if overlap < duration:
            raise ValueError(f'Concurrent coverage too short: {overlap:.3f}s')
        record.update(state='complete', overlap_seconds=overlap, summaries=summaries)
    except BaseException as error:
        record.update(state='incomplete', error=f'{type(error).__name__}: {error}')
        raise
    finally:
        # Only owned samplers are terminated; never kill the engine PIDs.
        for job in jobs:
            if job.poll() is None:
                job.terminate()
        for job in jobs:
            try:
                job.wait(timeout=3)
            except subprocess.TimeoutExpired:
                job.kill()
                job.wait()
        for handle in handles:
            handle.close()
        record['finished_at_unix'] = time.time()
        save(output / 'collection.json', record)
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pid', type=int, action='append', required=True)
    parser.add_argument('--duration', type=int, default=3600)
    parser.add_argument('--interval', type=int, default=5)
    parser.add_argument('--binary', type=Path, default=ROOT / 'tools/metrics/target/debug/engine-metrics')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--workload', required=True)
    parser.add_argument('--configuration', required=True)
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, signal.default_int_handler)
    try:
        result = collect(args.binary, args.pid, args.duration, args.interval, args.output,
                         args.workload, args.configuration)
    except (OSError, ValueError, RuntimeError, TimeoutError, KeyboardInterrupt) as error:
        parser.exit(1, f'Collection incomplete: {error}\n')
    print(f"Collection complete: {result['overlap_seconds']:.2f}s overlap. See {args.output / 'collection.json'}")


if __name__ == '__main__':
    main()
