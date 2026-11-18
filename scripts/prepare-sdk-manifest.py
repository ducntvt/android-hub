#!/usr/bin/env python3
"""Build a reviewable macOS ARM64 SDK plan from saved official XML; no install."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import xml.etree.ElementTree as ET

SOURCES = {
    'repository': 'https://dl.google.com/android/repository/',
    'image': 'https://dl.google.com/android/repository/sys-img/google_apis_playstore/',
}
PACKAGES = [
    ('repository', 'cmdline-tools;23.0', 'cmdline-tools/latest', True),
    ('repository', 'platform-tools', 'platform-tools', False),
    ('repository', 'emulator', 'emulator', True),
    ('image', 'system-images;android-36;google_apis_playstore;arm64-v8a',
     'system-images/android-36/google_apis_playstore/arm64-v8a', False),
]


def parse_xml(data):
    if len(data) > 8 * 1024 * 1024 or b'<!DOCTYPE' in data or b'<!ENTITY' in data:
        raise ValueError('Oversized XML or unsupported entity/doctype')
    root = ET.fromstring(data)
    for element in root.iter():
        element.tag = element.tag.split('}')[-1]
    return root


def revision(package):
    node = package.find('revision')
    if node is None or node.find('preview') is not None:
        raise ValueError('Missing/preview revision')
    parts = [int(node.findtext(key, '0')) for key in ('major', 'minor', 'micro')]
    if parts[0] < 1 or any(part < 0 for part in parts):
        raise ValueError('Invalid revision')
    return tuple(parts)


def select(root, source, name, destination, require_arm):
    candidates = []
    for package in root.findall('remotePackage'):
        channel = package.find('channelRef')
        if (package.get('path') != name or package.find('obsolete') is not None
                or channel is None or channel.get('ref') != 'channel-0'):
            continue
        candidates.append((revision(package), package))
    if not candidates:
        raise ValueError(f'Missing stable package: {name}')
    latest = max(item[0] for item in candidates)
    choices = [package for version, package in candidates if version == latest]
    if len(choices) != 1:
        raise ValueError(f'Ambiguous stable revision: {name}')
    package = choices[0]
    archives = []
    for archive in package.findall('./archives/archive'):
        host, arch = archive.findtext('host-os'), archive.findtext('host-arch')
        if host not in (None, 'macosx') or arch not in (None, 'aarch64'):
            continue
        if require_arm and (host != 'macosx' or arch != 'aarch64'):
            continue
        archives.append(archive)
    if len(archives) != 1:
        raise ValueError(f'Need exactly one macOS ARM64-compatible archive: {name}')
    complete = archives[0].find('complete')
    if complete is None:
        raise ValueError('Missing complete archive')
    size = int(complete.findtext('size', '0'))
    relative = complete.findtext('url', '')
    checksum = complete.find('checksum')
    if size <= 0 or size > 16 * 1024**3:
        raise ValueError('Invalid archive size')
    if (not re.fullmatch(r'[A-Za-z0-9_./-]+\.zip', relative)
            or PurePosixPath(relative).is_absolute() or '..' in PurePosixPath(relative).parts):
        raise ValueError('Unsafe archive URL')
    if checksum is None or checksum.get('type') not in ('sha1', 'sha256'):
        raise ValueError('Unsupported/missing published checksum')
    digest = (checksum.text or '').strip().lower()
    length = 40 if checksum.get('type') == 'sha1' else 64
    if not re.fullmatch('[0-9a-f]{' + str(length) + '}', digest):
        raise ValueError('Invalid published checksum')
    uses = package.find('uses-license')
    license_id = uses.get('ref') if uses is not None else None
    licenses = [node for node in root.findall('license') if node.get('id') == license_id]
    if not license_id or len(licenses) != 1:
        raise ValueError('Missing/ambiguous license reference')
    license_text = ''.join(licenses[0].itertext()).strip()
    if not license_text:
        raise ValueError('Empty license')
    dependencies = []
    for dependency in package.findall('./dependencies/dependency'):
        minimum = dependency.find('min-revision')
        dependencies.append({'package': dependency.get('path'),
                             'minimum_revision': {node.tag: node.text for node in minimum} if minimum is not None else None})
    return {'package': name, 'revision': '.'.join(map(str, latest)),
            'destination': destination, 'url': SOURCES[source] + relative, 'archive_bytes': size,
            'published_checksum': {'algorithm': checksum.get('type'), 'value': digest},
            'license': {'id': license_id, 'text_sha256': hashlib.sha256(license_text.encode()).hexdigest(),
                        'text': license_text, 'accepted': False}, 'dependencies': dependencies}


def prepare(repository, image):
    raw = {'repository': repository, 'image': image}
    roots = {key: parse_xml(value) for key, value in raw.items()}
    packages = [select(roots[source], source, name, destination, arm)
                for source, name, destination, arm in PACKAGES]
    return {'schema': 1, 'status': 'review_required', 'platform': 'macos-arm64',
            'sources': {key: {'url': SOURCES[key] + ('repository2-3.xml' if key == 'repository' else 'sys-img2-3.xml'),
                              'metadata_sha256': hashlib.sha256(value).hexdigest()} for key, value in raw.items()},
            'download_bytes': sum(package['archive_bytes'] for package in packages),
            'packages': packages,
            'limitations': ['No downloads, license acceptance, extraction or installation performed',
                            'Published SHA-1 checksums are recorded as SHA-1, not SHA-256 provenance',
                            'Dependency list needs resolution before execution; Java bootstrap is not included',
                            'Candidate engine version is not yet validated by Hub gameplay/benchmark',
                            'Compressed bytes do not establish extracted disk-space budget']}


def read_metadata(path):
    with path.open('rb') as handle:
        return handle.read(8 * 1024 * 1024 + 1)


def prepare_images(image):
    root = parse_xml(image)
    packages = [select(root, 'image',
                       f'system-images;android-{api};google_apis_playstore;arm64-v8a',
                       f'system-images/android-{api}/google_apis_playstore/arm64-v8a', False)
                for api in (31, 33, 34, 35, 36)]
    return {'schema': 1, 'platform': 'macos-arm64',
            'source': SOURCES['image'] + 'sys-img2-3.xml',
            'metadata_sha256': hashlib.sha256(image).hexdigest(),
            'packages': packages}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repository', type=Path)
    parser.add_argument('--images-only', action='store_true',
                        help='Prepare the Android 12–16 image catalog without common tools')
    parser.add_argument('--image', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    try:
        if not args.images_only and args.repository is None:
            raise ValueError('--repository is required unless --images-only is selected')
        plan = (prepare_images(read_metadata(args.image)) if args.images_only else
                prepare(read_metadata(args.repository), read_metadata(args.image)))
        with args.output.open('x') as handle:
            json.dump(plan, handle, indent=2, ensure_ascii=False)
            handle.write('\n')
    except (OSError, ValueError, ET.ParseError) as error:
        parser.exit(1, f'SDK plan rejected: {error}\n')
    total = sum(package['archive_bytes'] for package in plan['packages'])
    print(f"Prepared {len(plan['packages'])} packages; {total} compressed bytes. Review required; nothing installed.")


if __name__ == '__main__':
    main()
