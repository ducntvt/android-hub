"""Synthetic sampler fixtures only; these are not benchmark evidence."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import signal
import subprocess
import time
from unittest.mock import patch
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('collector', Path(__file__).with_name('collect-android-hub-metrics.py'))
collector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collector)


class CollectorTests(unittest.TestCase):
    def sampler(self, root, fail=False):
        path = root / 'synthetic-sampler'
        path.write_text(f'#!{sys.executable}\n' + '''import json, sys, time
from pathlib import Path
pid, count, interval = map(int, sys.argv[1:4])
''' + ('''if pid == 12345: sys.exit(7)
while True: time.sleep(1)
''' if fail else '''start = 100000 + (pid % 2)
rows = [{'event': 'start', 'schema': 1, 'pid': pid, 'rss_unit': 'KiB', 'samples_requested': count, 'interval_seconds': interval}]
rows += [{'event': 'sample', 'index': i, 'unix_seconds': start+i*interval, 'elapsed_seconds': i*interval, 'rss_kib': 1000, 'cpu_percent_ps': 10} for i in range(count)]
rows += [{'event': 'end', 'complete': True, 'samples_written': count}]
Path(sys.argv[4]).write_text(''.join(json.dumps(row)+'\\n' for row in rows))
'''))
        path.chmod(0o700)
        return path

    def test_settings_reject_invalid_or_oversized_requests(self):
        self.assertEqual(collector.settings([1, 2], 3600, 5), 722)
        for pids, duration, interval in [([], 60, 5), ([1, 1], 60, 5), ([0], 60, 5),
                                        ([1], 59, 5), ([1], 60, 6), ([1], 14400, 1)]:
            with self.assertRaises(ValueError):
                collector.settings(pids, duration, interval)

    def test_concurrent_trace_validation_and_refuse_existing_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            binary = self.sampler(root)
            output = root / 'synthetic-output'
            result = collector.collect(binary, [12345, 12346], 60, 5, output, 'SYNTHETIC TEST', 'fixture')
            self.assertEqual(result['state'], 'complete')
            self.assertEqual(result['overlap_seconds'], 64)
            original = (output / 'collection.json').read_bytes()
            with self.assertRaises(FileExistsError):
                collector.collect(binary, [12345], 60, 5, output, 'fixture', 'fixture')
            self.assertEqual((output / 'collection.json').read_bytes(), original)

    def test_failure_retains_incomplete_report_and_reaps_owned_samplers(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            output = root / 'synthetic-failure'
            with self.assertRaises(RuntimeError):
                collector.collect(self.sampler(root, True), [12345, 12346], 60, 5, output, 'fixture', 'fixture')
            report = json.loads((output / 'collection.json').read_text())
            self.assertEqual(report['state'], 'incomplete')
            for pid in report['sampler_pids']:
                with self.assertRaises(ProcessLookupError):
                    os.kill(pid, 0)

    def stalled_sampler(self, root):
        path = root / 'stalled-sampler'
        path.write_text(f'#!{sys.executable}\nimport time\ntime.sleep(120)\n')
        path.chmod(0o700)
        return path

    def test_timeout_reaps_samplers_and_retains_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            output = root / 'timeout'
            with patch.object(collector.time, 'monotonic', side_effect=[0, 1000]), self.assertRaises(TimeoutError):
                collector.collect(self.stalled_sampler(root), [12345, 12346], 60, 5, output, 'SYNTHETIC', 'fixture')
            report = json.loads((output / 'collection.json').read_text())
            self.assertEqual(report['state'], 'incomplete')
            self.assertIn('TimeoutError', report['error'])
            for pid in report['sampler_pids']:
                with self.assertRaises(ProcessLookupError):
                    os.kill(pid, 0)

    def test_signals_stop_only_samplers_and_keep_incomplete_evidence(self):
        for sig in (signal.SIGINT, signal.SIGTERM):
            with self.subTest(signal=sig), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                output = root / 'interrupted'
                engine = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(120)'])
                driver = None
                sampler_pids = []
                try:
                    driver = subprocess.Popen([
                        sys.executable, str(Path(collector.__file__)), '--pid', str(engine.pid),
                        '--duration', '60', '--interval', '2', '--binary', str(self.stalled_sampler(root)),
                        '--output', str(output), '--workload', 'SYNTHETIC SIGNAL TEST', '--configuration', 'fixture'
                    ], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                    deadline = time.monotonic() + 10
                    while time.monotonic() < deadline:
                        if (output / 'collection.json').exists():
                            report = json.loads((output / 'collection.json').read_text())
                            sampler_pids = report['sampler_pids']
                            if sampler_pids:
                                break
                        if driver.poll() is not None:
                            self.fail('Collector exited before sampler registration')
                        time.sleep(0.02)
                    self.assertTrue(sampler_pids, 'Sampler registration timed out')
                    driver.send_signal(sig)
                    driver.communicate(timeout=10)
                    self.assertEqual(driver.returncode, 1)
                    report = json.loads((output / 'collection.json').read_text())
                    self.assertEqual(report['state'], 'incomplete')
                    self.assertIn('KeyboardInterrupt', report['error'])
                    self.assertIsNone(engine.poll(), 'Collector must not stop engine')
                    for pid in sampler_pids:
                        with self.assertRaises(ProcessLookupError):
                            os.kill(pid, 0)
                finally:
                    if driver is not None and driver.poll() is None:
                        driver.kill()
                        driver.communicate()
                    engine.terminate()
                    engine.wait(timeout=5)


if __name__ == '__main__':
    unittest.main()
