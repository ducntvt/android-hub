import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('qa_package', Path(__file__).with_name('build-android-hub-qa.py'))
qa = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qa)


class LauncherTests(unittest.TestCase):
    def test_window_size_is_bounded_and_never_shell_text(self):
        self.assertIn('export ANDROID_HUB_WINDOW_SIZE=1000x640\n', qa.render_launcher('1000x640'))
        self.assertEqual(qa.render_launcher(), qa.LAUNCHER)
        for value in ['999x640', '1000x639', '1000x640;touch /tmp/x', '$(id)', '1000x99999']:
            with self.assertRaises(ValueError):
                qa.render_launcher(value)

    def test_spaces_isolation_and_arguments(self):
        # Exercise only a shell fixture, never the GUI or actual guest data.
        with tempfile.TemporaryDirectory(prefix='hub qa ') as directory:
            root = Path(directory).resolve()
            binary_dir = root / 'Android Hub QA.app/Contents/MacOS'
            binary_dir.mkdir(parents=True)
            launcher = binary_dir / 'qa'
            launcher.write_text(qa.LAUNCHER)
            binary = binary_dir / 'android-hub'
            binary.write_text('#!/bin/sh\nprintf "%s\\n" "$ANDROID_HUB_ROOT" "${ANDROID_SDK_ROOT-unset}" "${P0_JAVA_HOME-unset}" "${ANDROID_HUB_JAVA_HOME-unset}" "$@"\n')
            binary.chmod(0o755)
            env = dict(os.environ, ANDROID_HUB_ROOT='/wrong', ANDROID_SDK_ROOT='/wrong', P0_JAVA_HOME='/wrong', ANDROID_HUB_JAVA_HOME='/wrong')
            result = subprocess.run(['/bin/sh', str(launcher), 'one argument'], env=env, capture_output=True, text=True, check=True)
            self.assertEqual(result.stdout.splitlines(), [str(root / 'data'), 'unset', 'unset', 'unset', 'one argument'])

    def test_existing_destination_is_never_replaced(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'binary'
            source.write_text('fixture')
            with self.assertRaises(FileExistsError):
                qa.package(root, source)
            self.assertEqual(source.read_text(), 'fixture')
