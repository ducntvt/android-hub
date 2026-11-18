import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('gate', Path(__file__).with_name('verify-p0.py'))
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class EvidenceGateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        runs = {}
        for name, (count, duration) in gate.SCENARIOS.items():
            traces = []
            for index in range(count):
                file = f'{name}-{index}.jsonl'
                traces.append(file)
                rows = [{'event': 'start', 'schema': 1, 'rss_unit': 'KiB', 'pid': 100 + index,
                         'samples_requested': duration // 5 + 1, 'interval_seconds': 5}]
                rows += [{'event': 'sample', 'index': n, 'elapsed_seconds': n * 5,
                          'unix_seconds': 10000 + n * 5, 'rss_kib': 1024,
                          'cpu_percent_ps': 10} for n in range(duration // 5 + 1)]
                rows += [{'event': 'end', 'complete': True, 'samples_written': duration // 5 + 1}]
                (self.root / file).write_text('\n'.join(json.dumps(r) for r in rows))
            runs[name] = {'configuration': 'synthetic fixture', 'workload': 'synthetic only', 'process_traces': traces}
        self.manifest = dict(schema=1, runs=runs, **{k: 'fixture' for k in ('host', 'engine', 'image', 'renderer', 'gapps', 'game_versions')})
        self.save()
        (self.root / 'observation.txt').write_text('Synthetic test fixture, not actual gameplay evidence')
        self.checks = {k: {'status': 'pass', 'reviewer': 'test fixture', 'reviewed_at': '2026-11-13T00:00:00Z',
                          'observation': 'synthetic', 'evidence': ['observation.txt']} for k in gate.CHECKS}
        self.save_acceptance()

    def save(self):
        (self.root / 'measurements.json').write_text(json.dumps(self.manifest))

    def save_acceptance(self):
        (self.root / 'acceptance.json').write_text(json.dumps({'schema': 1, 'checks': self.checks}))

    def test_valid_synthetic_records(self):
        self.assertEqual(gate.acceptance(self.root)['dual']['overlap_seconds'], 3600)

    def test_pending_or_missing_evidence_never_passes(self):
        self.checks['hunt_royale']['status'] = 'pending'
        self.save_acceptance()
        with self.assertRaises(ValueError): gate.acceptance(self.root)
        self.checks['hunt_royale']['status'] = 'pass'
        self.save_acceptance()
        (self.root / 'observation.txt').unlink()
        with self.assertRaises(ValueError): gate.acceptance(self.root)

    def test_two_nonoverlapping_runs_are_not_simultaneous(self):
        path = self.root / 'dual-1.jsonl'
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        for row in rows[1:-1]: row['unix_seconds'] += 4000
        path.write_text('\n'.join(json.dumps(r) for r in rows))
        with self.assertRaisesRegex(ValueError, 'simultaneous'): gate.measurements(self.root)

    def test_duplicate_pid_and_trace_are_rejected(self):
        self.manifest['runs']['dual']['process_traces'][1] = 'dual-0.jsonl'
        self.save()
        with self.assertRaises(ValueError): gate.measurements(self.root)

    def test_traversal_and_symlink_escape_are_rejected(self):
        with self.assertRaises(ValueError): gate.artifact(self.root, '../outside')
        with tempfile.TemporaryDirectory() as other:
            outside = Path(other) / 'data'
            outside.write_text('test')
            (self.root / 'link').symlink_to(outside)
            with self.assertRaises(ValueError): gate.artifact(self.root, 'link')

    def test_acceptance_cannot_skip_measurements(self):
        (self.root / 'measurements.json').unlink()
        with self.assertRaises(OSError): gate.acceptance(self.root)


if __name__ == '__main__': unittest.main()
