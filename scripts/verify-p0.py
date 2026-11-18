#!/usr/bin/env python3
"""Read-only P0 evidence gate. Validates records, never proves gameplay itself."""
import argparse
import importlib.util
import json
from datetime import datetime
from pathlib import Path

spec = importlib.util.spec_from_file_location('metrics', Path(__file__).with_name('p0-metrics-summary.py'))
metrics = importlib.util.module_from_spec(spec)
spec.loader.exec_module(metrics)
CHECKS = ('host_manifest', 'component_licenses', 'host_integration', 'gpu_renderer',
          'play_store', 'top_heroes', 'hunt_royale', 'dual_gameplay', 'audio_network',
          'isolation_restart', 'persistence', 'guest_fps', 'host_presentation',
          'memory_pressure', 'minimized_behavior', 'engine_comparison', 'engine_decision')
SCENARIOS = {'single': (1, 60), 'dual': (2, 3600), 'dual_minimized': (2, 60)}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def nonempty(value, name):
    require(isinstance(value, str) and bool(value.strip()), f'Missing {name}')
    return value


def artifact(root, value):
    relative = Path(nonempty(value, 'evidence path'))
    require(not relative.is_absolute() and '..' not in relative.parts, 'Evidence must stay inside report directory')
    path = (root / relative).resolve()
    require(path.is_relative_to(root.resolve()), 'Evidence escapes report directory')
    require(path.is_file() and path.stat().st_size > 0, f'Missing/empty evidence: {relative}')
    return path


def load(path):
    require(path.stat().st_size <= 2 * 1024 * 1024, 'Manifest exceeds 2 MiB')
    value = json.loads(path.read_text())
    require(isinstance(value, dict) and type(value.get('schema')) is int and value['schema'] == 1,
            'Unsupported evidence schema')
    return value


def timestamp(value):
    parsed = datetime.fromisoformat(nonempty(value, 'review timestamp').replace('Z', '+00:00'))
    require(parsed.utcoffset() is not None, 'Review timestamp needs timezone')


def measurements(root):
    manifest = load(root / 'measurements.json')
    for field in ('host', 'engine', 'image', 'renderer', 'gapps', 'game_versions'):
        nonempty(manifest.get(field), field)
    runs = manifest.get('runs')
    require(isinstance(runs, dict) and set(runs) == set(SCENARIOS), 'Need single, dual and dual_minimized runs')
    report = {}
    for name, (count, minimum) in SCENARIOS.items():
        run = runs[name]
        require(isinstance(run, dict), f'Invalid run: {name}')
        nonempty(run.get('configuration'), f'{name} configuration')
        nonempty(run.get('workload'), f'{name} workload')
        traces = run.get('process_traces')
        require(isinstance(traces, list) and len(traces) == count, f'{name} requires {count} process traces')
        pids, intervals, summaries, paths = set(), [], [], set()
        for trace in traces:
            path = artifact(root, trace)
            require(path not in paths, f'{name}: duplicate trace')
            paths.add(path)
            rows = metrics.load(path)
            summary = metrics.summarize(rows, minimum)
            require(rows[0]['interval_seconds'] <= 5, f'{name}: sampling interval must be at most 5 seconds')
            require(summary['pid'] not in pids, f'{name}: duplicate PID')
            pids.add(summary['pid'])
            times = [row['unix_seconds'] for row in rows[1:-1]]
            require(all(b > a for a, b in zip(times, times[1:])), 'Non-monotonic wall clock')
            require(abs((times[-1] - times[0]) - summary['measured_span_seconds']) <= 2,
                    'Wall/monotonic duration mismatch')
            intervals.append((times[0], times[-1]))
            summaries.append(summary)
        overlap = min(end for _, end in intervals) - max(start for start, _ in intervals)
        require(overlap >= minimum, f'{name}: insufficient simultaneous coverage ({overlap}s)')
        report[name] = {'overlap_seconds': overlap, 'processes': summaries}
    return report


def acceptance(root):
    # Acceptance cannot bypass the benchmark gate.
    result = measurements(root)
    manifest = load(root / 'acceptance.json')
    checks = manifest.get('checks')
    require(isinstance(checks, dict) and set(checks) == set(CHECKS), 'Acceptance check set incomplete/unknown')
    for name in CHECKS:
        item = checks[name]
        require(isinstance(item, dict) and item.get('status') == 'pass', f'Pending/failed acceptance: {name}')
        nonempty(item.get('reviewer'), f'{name} reviewer')
        timestamp(item.get('reviewed_at'))
        nonempty(item.get('observation'), f'{name} observation')
        evidence = item.get('evidence')
        require(isinstance(evidence, list) and evidence, f'{name}: evidence missing')
        for reference in evidence:
            artifact(root, reference)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', choices=('measurements', 'acceptance'), required=True)
    parser.add_argument('--report-dir', type=Path, required=True)
    args = parser.parse_args()
    try:
        result = (measurements if args.check == 'measurements' else acceptance)(args.report_dir)
    except (OSError, ValueError, TypeError, KeyError, IndexError) as error:
        parser.exit(1, f'P0 rejected: {error}\n')
    print(json.dumps({'records_valid': True, 'check': args.check, 'measurements': result,
                     'limitation': 'Structural validation only. Human observations and artifact contents require independent review.'}, indent=2))


if __name__ == '__main__':
    main()
