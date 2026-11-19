#!/usr/bin/env python3
"""Read product metadata and validate releases; never publish or modify Git."""
import argparse
import datetime
import plistlib
import re
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = re.compile(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z.-]+))?')


def product_version(root=ROOT):
    with (root / 'Cargo.toml').open('rb') as source:
        version = tomllib.load(source)['workspace']['package']['version']
    match = VERSION.fullmatch(version)
    if not match:
        raise ValueError('Expected MAJOR.MINOR.PATCH with an optional prerelease (no build metadata).')
    if match[4]:
        for part in match[4].split('.'):
            if not part or (part.isdigit() and len(part) > 1 and part.startswith('0')):
                raise ValueError('Invalid SemVer prerelease identifier.')
    return version


def bundle_version(version):
    return version.split('-', 1)[0]


def release_notes(changelog, version):
    sections = re.split(r'^## ', changelog, flags=re.M)[1:]
    matches = []
    for section in sections:
        header, _, body = section.partition('\n')
        match = re.fullmatch(r'\[' + re.escape(version) + r'\] - (\d{4}-\d{2}-\d{2})', header)
        if match:
            date = datetime.date.fromisoformat(match[1])
            if date > datetime.date.today():
                raise ValueError('Release date cannot be in the future.')
            matches.append(body.strip())
    if len(matches) != 1 or not matches[0]:
        raise ValueError(f'Expected exactly one nonempty changelog section: ## [{version}] - YYYY-MM-DD')
    return matches[0] + '\n'


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()


def validate_release(tag, root=ROOT):
    version = product_version(root)
    if tag != f'v{version}':
        raise ValueError(f'Tag must match Cargo version exactly: v{version}')
    commit = git(root, 'rev-parse', '--verify', f'refs/tags/{tag}^{{commit}}')
    if commit != git(root, 'rev-parse', 'HEAD'):
        raise ValueError('Check out the tagged commit before validating.')
    subprocess.run(
        ['git', '-C', str(root), 'merge-base', '--is-ancestor', commit, 'refs/remotes/origin/main'],
        check=True,
    )
    return release_notes((root / 'CHANGELOG.md').read_text(), version)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    commands.add_parser('bundle-version')
    commands.add_parser('product-version')
    validate = commands.add_parser('validate')
    validate.add_argument('tag')
    validate.add_argument('--notes', type=Path)
    verify = commands.add_parser('verify-bundle')
    verify.add_argument('plist', type=Path)
    args = parser.parse_args()
    try:
        if args.command == 'product-version':
            print(product_version())
        elif args.command == 'bundle-version':
            print(bundle_version(product_version()))
        elif args.command == 'verify-bundle':
            with args.plist.open('rb') as source:
                metadata = plistlib.load(source)
            expected = bundle_version(product_version())
            for key in ('CFBundleShortVersionString', 'CFBundleVersion'):
                if metadata.get(key) != expected:
                    raise ValueError(f'{key} must equal {expected}')
            print(f'Bundle version matches Cargo: {expected}')
        else:
            notes = validate_release(args.tag)
            if args.notes:
                args.notes.write_text(notes)
            print(f'Release validated: {args.tag}')
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f'Release metadata error: {error}\n')


if __name__ == '__main__':
    main()
