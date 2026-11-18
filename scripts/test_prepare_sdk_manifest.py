import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('sdk_plan', Path(__file__).with_name('prepare-sdk-manifest.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def xml(name='emulator', host='macosx', arch='aarch64', channel='channel-0', url='emulator.zip', checksum='a'*40):
    return f'''<repository><license id="license">Fixture license</license>
<remotePackage path="{name}"><revision><major>37</major><minor>2</minor><micro>13</micro></revision>
<channelRef ref="{channel}"/><uses-license ref="license"/><archives><archive>
<host-os>{host}</host-os><host-arch>{arch}</host-arch><complete><size>1024</size>
<checksum type="sha1">{checksum}</checksum><url>{url}</url></complete></archive></archives>
</remotePackage></repository>'''.encode()


def select(data):
    return module.select(module.parse_xml(data), 'repository', 'emulator', 'emulator', True)


class ManifestTests(unittest.TestCase):
    def test_image_catalog_contains_exactly_the_five_requested_packages(self):
        root = module.parse_xml(xml())
        root.remove(root.find('remotePackage'))
        for api in (31, 33, 34, 35, 36):
            name = f'system-images;android-{api};google_apis_playstore;arm64-v8a'
            root.append(module.parse_xml(xml(name=name)).find('remotePackage'))
        raw = module.ET.tostring(root)
        plan = module.prepare_images(raw)
        self.assertEqual(len(plan['packages']), 5)
        self.assertEqual([p['package'].split(';')[1] for p in plan['packages']],
                         ['android-31', 'android-33', 'android-34', 'android-35', 'android-36'])
        self.assertTrue(all(not p['license']['accepted'] for p in plan['packages']))
        root.remove(root.findall('remotePackage')[0])
        with self.assertRaises(ValueError):
            module.prepare_images(module.ET.tostring(root))

    def test_pins_stable_arm64_and_preserves_checksum_algorithm_and_license(self):
        package = select(xml())
        self.assertEqual(package['revision'], '37.2.13')
        self.assertEqual(package['published_checksum'], {'algorithm': 'sha1', 'value': 'a'*40})
        self.assertFalse(package['license']['accepted'])
        self.assertEqual(len(package['license']['text_sha256']), 64)
        self.assertEqual(package['url'], 'https://dl.google.com/android/repository/emulator.zip')

    def test_rejects_wrong_platform_preview_and_missing_license(self):
        for data in [xml(host='linux'), xml(arch='x64'), xml(channel='channel-1'),
                     xml().replace(b'ref="license"', b'ref="missing"'),
                     xml().replace(b'<revision>', b'<revision><preview>1</preview>')]:
            with self.assertRaises(ValueError):
                select(data)

    def test_rejects_unsafe_urls_checksum_and_duplicate_archives(self):
        for url in ['../payload.zip', '/absolute.zip', 'https://elsewhere/payload.zip',
                    'payload.zip?secret', '%2e%2e/payload.zip']:
            with self.assertRaises(ValueError):
                select(xml(url=url))
        with self.assertRaises(ValueError):
            select(xml(checksum='broken'))
        root = module.parse_xml(xml())
        archives = root.find('./remotePackage/archives')
        archives.append(archives[0])
        with self.assertRaises(ValueError):
            module.select(root, 'repository', 'emulator', 'emulator', True)

    def test_rejects_entities_and_oversized_metadata(self):
        for data in [b'<!DOCTYPE repository><repository/>', b'<!ENTITY foo "bar"><repository/>', b' '*(8*1024*1024+1)]:
            with self.assertRaises(ValueError):
                module.parse_xml(data)

    def test_requires_all_four_packages_before_producing_plan(self):
        with self.assertRaises(ValueError):
            module.prepare(xml(), xml())


if __name__ == '__main__':
    unittest.main()
