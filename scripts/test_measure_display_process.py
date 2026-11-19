import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('measure_display_process', Path(__file__).with_name('measure-display-process.py'))
measure = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measure)


class DisplayMeasurementTests(unittest.TestCase):
    def record(self):
        return measure.parse_process('42 Thu Nov 19 14:00:00 2026 12.5 1024 /Applications/Android Hub.app/Contents/MacOS/android-hub', 42)

    def test_parser_preserves_executable_spaces_and_rejects_wrong_identity(self):
        self.assertIn('Android Hub.app', self.record()['executable'])
        self.assertEqual(self.record()['cpu_percent'], 12.5)
        for line in ['43 Thu Nov 19 14:00:00 2026 0 1 /bin/test',
                     '42 Thu Nov 19 14:00:00 2026 nan 1 /bin/test', '']:
            with self.assertRaises(ValueError):
                measure.parse_process(line, 42)

    def test_bounded_collection_and_no_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'run.jsonl'
            now = [0.0]
            def sleep(delay):
                now[0] += delay
            measure.collect([42], 2, path, lambda _: self.record(), lambda: now[0], sleep)
            rows = [json.loads(line) for line in path.read_text().splitlines()]
            self.assertEqual(rows[-1]['samples'], 2)
            self.assertTrue(rows[-1]['complete'])
            original = path.read_bytes()
            with self.assertRaises(FileExistsError):
                measure.collect([42], 2, path, lambda _: self.record())
            self.assertEqual(original, path.read_bytes())

    def test_reused_pid_records_incomplete_without_sampling_replacement(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'run.jsonl'
            calls = [0]
            def read(_):
                calls[0] += 1
                record = self.record()
                if calls[0] > 1:
                    record['started'] = 'replacement'
                return record
            with self.assertRaises(ValueError):
                measure.collect([42], 1, path, read)
            rows = [json.loads(line) for line in path.read_text().splitlines()]
            self.assertFalse(rows[-1]['complete'])
            self.assertEqual(rows[-1]['samples'], 0)

    def test_invalid_scope_never_creates_output(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'run.jsonl'
            for pids, seconds in [([], 1), ([42, 42], 1), ([0], 1), ([42], 121)]:
                with self.assertRaises(ValueError):
                    measure.collect(pids, seconds, path)
                self.assertFalse(path.exists())
