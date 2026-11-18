"""Release gates use disposable Git repositories, never the developer's tags."""
import datetime
import subprocess
import tempfile
import unittest
from pathlib import Path

import release


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='android-hub-release-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.write_version('0.1.0')
        (self.root / 'CHANGELOG.md').write_text(
            '# Changelog\n\n## Unreleased\n\nNext work.\n\n'
            '## [0.1.0] - 2026-01-01\n\n- Initial release.\n\n'
            '## [0.0.1] - 2025-12-01\n\n- Old notes.\n'
        )
        self.git('init', '-b', 'main')
        self.git('add', '.')
        self.git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                 '-c', 'commit.gpgsign=false', 'commit', '-m', 'Fixture')
        self.git('update-ref', 'refs/remotes/origin/main', 'HEAD')
        self.git('-c', 'tag.gpgsign=false', 'tag', 'v0.1.0')

    def git(self, *args):
        return release.git(self.root, *args)

    def write_version(self, version):
        (self.root / 'Cargo.toml').write_text(f'[workspace.package]\nversion = "{version}"\n')

    def test_release_extracts_only_matching_notes(self):
        self.assertEqual(release.validate_release('v0.1.0', self.root), '- Initial release.\n')

    def test_tag_must_match_version(self):
        for tag in ('v0.2.0', '0.1.0', '--help', 'v0.1.0;echo bad'):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                release.validate_release(tag, self.root)

    def test_rejects_tag_outside_main(self):
        self.git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                 '-c', 'commit.gpgsign=false', 'commit', '--allow-empty', '-m', 'Unmerged')
        self.git('-c', 'tag.gpgsign=false', 'tag', '-f', 'v0.1.0')
        with self.assertRaises(subprocess.CalledProcessError):
            release.validate_release('v0.1.0', self.root)

    def test_rejects_checkout_different_from_tag(self):
        self.git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                 '-c', 'commit.gpgsign=false', 'commit', '--allow-empty', '-m', 'Later')
        with self.assertRaises(ValueError):
            release.validate_release('v0.1.0', self.root)

    def test_annotated_tag_on_main_ancestor_is_allowed(self):
        self.git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                 '-c', 'tag.gpgsign=false', 'tag', '-fa', 'v0.1.0', '-m', 'Release')
        self.git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                 '-c', 'commit.gpgsign=false', 'commit', '--allow-empty', '-m', 'Later main')
        self.git('update-ref', 'refs/remotes/origin/main', 'HEAD')
        self.git('checkout', '--detach', 'v0.1.0')
        self.assertEqual(release.validate_release('v0.1.0', self.root), '- Initial release.\n')

    def test_requires_unique_nonempty_dated_notes(self):
        valid = '## [0.1.0] - 2026-01-01\n\n- Notes.\n'
        future = datetime.date.today() + datetime.timedelta(days=1)
        for notes in ('## Unreleased\nNotes', '## [0.1.0]\nNotes',
                      '## [0.1.0] - 2026-01-01\n\n', valid + valid,
                      '## [0.1.0] - 2026-02-30\nNotes',
                      f'## [0.1.0] - {future}\nNotes'):
            with self.subTest(notes=notes), self.assertRaises(ValueError):
                release.release_notes(notes, '0.1.0')

    def test_prerelease_uses_numeric_bundle_version(self):
        for version in ('0.1.0', '0.1.0-beta.1', '0.1.0-rc.2'):
            self.write_version(version)
            self.assertEqual(release.product_version(self.root), version)
            self.assertEqual(release.bundle_version(version), '0.1.0')

    def test_rejects_invalid_versions_and_build_metadata(self):
        for version in ('01.2.3', '1.2', '1.2.3-beta..1', '1.2.3-01', '1.2.3+abc', '1.2.3-'):
            self.write_version(version)
            with self.subTest(version=version), self.assertRaises(ValueError):
                release.product_version(self.root)


if __name__ == '__main__':
    unittest.main()
