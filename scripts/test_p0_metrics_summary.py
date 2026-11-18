import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('metrics', Path(__file__).with_name('p0-metrics-summary.py'))
metrics = importlib.util.module_from_spec(spec)
spec.loader.exec_module(metrics)


class SummaryTests(unittest.TestCase):
    def setUp(self):
        self.rows = [
            {'event': 'start', 'schema': 1, 'pid': 123, 'rss_unit': 'KiB',
             'samples_requested': 2, 'interval_seconds': 10},
            {'event': 'sample', 'index': 0, 'unix_seconds': 100, 'elapsed_seconds': 0.1,
             'rss_kib': 1000, 'cpu_percent_ps': 200},
            {'event': 'sample', 'index': 1, 'unix_seconds': 110, 'elapsed_seconds': 10.2,
             'rss_kib': 1200, 'cpu_percent_ps': 50},
            {'event': 'end', 'complete': True, 'samples_written': 2},
        ]

    def test_summarizes_with_explicit_units(self):
        summary = metrics.summarize(self.rows, min_duration=10)
        self.assertEqual(summary['rss_kib']['sample_mean'], 1100)
        self.assertEqual(summary['cpu_percent_ps']['peak_sample'], 200)

    def test_rejects_missing_or_failed_end(self):
        with self.assertRaises(ValueError):
            metrics.summarize(self.rows[:-1])
        self.rows[-1]['complete'] = False
        with self.assertRaises(ValueError):
            metrics.summarize(self.rows)

    def test_rejects_corrupt_metrics(self):
        for index, key, value in [(1, 'rss_kib', -1), (1, 'cpu_percent_ps', float('nan')),
                                  (1, 'rss_kib', True), (2, 'index', 0),
                                  (2, 'elapsed_seconds', 0), (2, 'elapsed_seconds', 40),
                                  (0, 'samples_requested', 3), (3, 'samples_written', 1)]:
            rows = copy.deepcopy(self.rows)
            rows[index][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                metrics.summarize(rows)

    def test_short_smoke_cannot_be_used_as_soak_test(self):
        with self.assertRaisesRegex(ValueError, 'Insufficient measured duration'):
            metrics.summarize(self.rows, min_duration=3600)


if __name__ == '__main__':
    unittest.main()
