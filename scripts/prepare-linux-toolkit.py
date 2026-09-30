#!/usr/bin/env python3
"""Seal a staged Linux toolkit, or complete its missing official Rust toolchain.

Input is the payload layout documented in tests/build/README-linux-toolkit.md.
This command does not install system packages or build the application.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
import urllib.request

from build_evidence import _check_frontend, _check_inventory, _check_toolkit_archive, validate_toolkit

ROOT = Path(__file__).resolve().parents[1]
TARGET = 'x86_64-unknown-linux-gnu'
RUST_COMPONENTS = ('rustc', 'cargo', 'rust-std', 'rustfmt-preview', 'clippy-preview')


def sha256(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def identity(root, path):
    path = Path(path)
    for parent in path.parents:
        if parent == root:
            break
        if parent.is_symlink() or not parent.is_dir():
            raise ValueError(f'toolkit file has a linked or unavailable parent: {path}')
    if not path.resolve().is_relative_to(root.resolve()):
        raise ValueError(f'toolkit file/link escapes its root: {path}')
    return dict(path=path.relative_to(root).as_posix(), sha256=sha256(path))


def plain_directory(root, relative, *, create=False):
    current = root
    for part in PurePosixPath(relative).parts:
        current = current / part
        if current.is_symlink() or (current.exists() and not current.is_dir()):
            raise ValueError(f'toolkit directory must be an ordinary directory: {current}')
        if not current.exists():
            if not create:
                raise ValueError(f'toolkit directory is unavailable: {current}')
            current.mkdir()
    return current


def check_layout(toolkit):
    if toolkit.is_symlink() or not toolkit.is_dir():
        raise ValueError('toolkit root must be an ordinary directory')
    for relative in ('payload', 'payload/bin', 'payload/lib', 'payload/frontend',
                     'payload/frontend/node_modules', 'payload/native', 'payload/cargo', 'payload/vendor'):
        plain_directory(toolkit, relative)


def atomic_json(path, value):
    with tempfile.NamedTemporaryFile(mode='w', dir=path.parent, delete=False) as output:
        temporary = Path(output.name)
        try:
            json.dump(value, output, indent=2)
            output.write('\n')
            output.flush()
            temporary.replace(path)
        finally:
            temporary.unlink(missing_ok=True)


def inventory(directory):
    result = {}
    for path in sorted(directory.rglob('*')):
        if path.is_symlink() and not path.is_file():
            raise ValueError(f'unsupported inventory directory or broken link: {path}')
        if path.is_file():
            if not path.resolve().is_relative_to(directory.resolve()):
                raise ValueError(f'inventory file link escapes its directory: {path}')
            result[path.relative_to(directory).as_posix()] = sha256(path)
    return result


def checked_rust_version(value):
    if not re.fullmatch(r'\d+\.\d+\.\d+', value):
        raise ValueError('Rust version must be an exact stable X.Y.Z version')
    if tuple(map(int, value.split('.'))) < (1, 91, 0):
        raise ValueError('the patched workspace requires Rust 1.91.0 or newer')
    return value


def safe_extract_rust(archive, destination):
    """Rust component packages need only regular files/directories, never links."""
    destination = Path(destination)
    if destination.exists():
        raise ValueError('Rust extraction requires a fresh destination')
    with tarfile.open(archive) as bundle:
        names = set()
        for member in bundle.getmembers():
            path = PurePosixPath(member.name)
            if (not member.name or path.is_absolute() or '..' in path.parts
                    or '\\' in member.name or member.name.rstrip('/') != str(path)
                    or str(path) in names or not (member.isfile() or member.isdir())):
                raise ValueError(f'unsafe or unsupported Rust archive member: {member.name}')
            names.add(str(path))
        destination.mkdir(parents=True)
        bundle.extractall(destination, filter='data')


def download(url, path=None, expected=None):
    if path is not None and path.exists() and expected and sha256(path) == expected:
        return path
    request = urllib.request.Request(url, headers={'User-Agent': 'AgenticInternetToolkit/1'})
    with urllib.request.urlopen(request, timeout=90) as response:
        if path is None:
            return response.read()
        with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as temporary:
            temporary_path = Path(temporary.name)
            try:
                shutil.copyfileobj(response, temporary, 1024 * 1024)
                temporary.flush()
                if expected is not None and sha256(temporary_path) != expected:
                    raise ValueError(f'official component checksum mismatch: {url}')
                temporary_path.replace(path)
            finally:
                temporary_path.unlink(missing_ok=True)
    return path


def complete_rust(toolkit, version, target):
    check_layout(toolkit)
    if target != TARGET:
        raise ValueError('the staged native SDK currently supports x86_64-unknown-linux-gnu only')
    version = checked_rust_version(version)
    prefix = toolkit / 'payload/lib/rust'
    if prefix.exists() or prefix.is_symlink():
        raise ValueError('Rust payload already exists; refuse to overwrite a prepared toolchain')
    for executable in ('rustc', 'cargo', 'rustfmt', 'clippy-driver', 'cargo-clippy', 'cargo-fmt', 'rustdoc'):
        launcher = toolkit / 'payload/bin' / executable
        if launcher.exists() or launcher.is_symlink():
            raise ValueError(f'refusing to overwrite a staged Rust launcher: {launcher}')
    downloads = plain_directory(toolkit, 'downloads', create=True)
    url = f'https://static.rust-lang.org/dist/channel-rust-{version}.toml'
    encoded = download(url)
    expected = download(url + '.sha256').decode().split()[0]
    if hashlib.sha256(encoded).hexdigest() != expected:
        raise ValueError('official Rust channel manifest checksum mismatch')
    channel = tomllib.loads(encoded.decode())
    entries = {name: channel['pkg'][name]['target'][target] for name in RUST_COMPONENTS}
    for entry in entries.values():
        download(entry['xz_url'], downloads / entry['xz_url'].rsplit('/', 1)[-1], entry['xz_hash'])
    # Never expose a partially installed toolchain at the final payload path.
    with tempfile.TemporaryDirectory(prefix='rust-stage-', dir=toolkit) as temporary:
        stage = Path(temporary)
        installed = stage / 'installed'
        for name, entry in entries.items():
            archive = downloads / entry['xz_url'].rsplit('/', 1)[-1]
            unpack = stage / name
            safe_extract_rust(archive, unpack)
            installers = list(unpack.glob('*/install.sh'))
            if len(installers) != 1:
                raise ValueError(f'official Rust installer unavailable: {name}')
            subprocess.run(['sh', str(installers[0]), f'--prefix={installed}', '--disable-ldconfig'], check=True)
            with tarfile.open(archive) as bundle:
                for member in bundle:
                    parts = PurePosixPath(member.name).parts
                    if not member.isfile() or len(parts) < 3 or parts[2] == 'manifest.in':
                        continue
                    destination = installed.joinpath(*parts[2:])
                    if not destination.is_file():
                        raise ValueError(f'installed Rust file is missing: {destination}')
                    original = bundle.extractfile(member).read()
                    if hashlib.sha256(destination.read_bytes()).digest() != hashlib.sha256(original).digest():
                        raise ValueError(f'installed Rust file differs from official component: {destination}')
        for executable in ('rustc', 'cargo', 'rustfmt', 'clippy-driver'):
            subprocess.run([str(installed / 'bin' / executable), '--version'], check=True)
        installed.rename(prefix)
    for executable in ('rustc', 'cargo', 'rustfmt', 'clippy-driver', 'cargo-clippy', 'cargo-fmt', 'rustdoc'):
        (toolkit / 'payload/bin' / executable).symlink_to('../lib/rust/bin/' + executable)
    provenance = plain_directory(toolkit, 'payload/provenance', create=True)
    atomic_json(provenance / 'rust.json', dict(
        schema=1, manifestUrl=url, manifestSha256=expected, components=entries))


def tool_environment(toolkit):
    payload = toolkit / 'payload'
    native = payload / 'native'
    env = dict(os.environ)
    env['PATH'] = os.pathsep.join(map(str, (payload / 'bin', native / 'bin', native / 'usr/bin',
        native / 'usr/lib/llvm-18/bin'))) + os.pathsep + env['PATH']
    env['LD_LIBRARY_PATH'] = os.pathsep.join(map(str, (native / 'usr/lib/x86_64-linux-gnu',
        native / 'usr/lib/llvm-18/lib', payload / 'lib/rust/lib')))
    return env


def manifest_for(toolkit, target):
    check_layout(toolkit)
    if target != TARGET:
        raise ValueError('the staged native SDK currently supports x86_64-unknown-linux-gnu only')
    payload = toolkit / 'payload'
    env = tool_environment(toolkit)
    tools, files = {}, {}
    for name in ('node', 'npm', 'rustc', 'cargo', 'forge', 'anvil', 'cast', 'solc', 'python3'):
        binary = payload / 'bin' / name
        if binary.exists() or binary.is_symlink():
            # Validate containment before invoking any provided executable.
            files[name] = identity(toolkit, binary)
            tools[name] = subprocess.check_output([str(binary), '--version'], env=env, text=True).strip()
            if identity(toolkit, binary) != files[name]:
                raise ValueError(f'provided tool changed while collecting its version: {name}')
    native_paths = {
        'cc': 'native/bin/cc', 'pkg-config': 'native/bin/pkg-config',
        'openssl': 'native/usr/lib/x86_64-linux-gnu/pkgconfig/openssl.pc',
        'gtk+-3.0': 'native/usr/lib/x86_64-linux-gnu/pkgconfig/gtk+-3.0.pc',
        'webkit2gtk-4.1': 'native/usr/lib/x86_64-linux-gnu/pkgconfig/webkit2gtk-4.1.pc',
    }
    frontend = dict(packageLockPath='payload/frontend/package-lock.json',
        packageLockSha256=sha256(payload / 'frontend/package-lock.json'),
        nodeModulesPath='payload/frontend/node_modules', nodeModulesFiles=inventory(payload / 'frontend/node_modules'))
    cargo = dict(locked=True, offline=True, lockPath='payload/Cargo.lock', lockSha256=sha256(payload / 'Cargo.lock'),
        vendorPath='payload/vendor', vendorFiles=inventory(payload / 'vendor'),
        configPath='payload/cargo/config.toml', configSha256=sha256(payload / 'cargo/config.toml'))
    checks = []
    for name, check in [('frontend', lambda: _check_frontend(toolkit, frontend, target, ROOT)),
                        ('vendor-inventory', lambda: _check_inventory(payload / 'vendor', cargo['vendorFiles']))]:
        try:
            check()
            checks.append(dict(name=name, status='passed'))
        except (OSError, ValueError, KeyError, TypeError) as error:
            checks.append(dict(name=name, status='failed', error=f'{type(error).__name__}: {error}'))
    return dict(schema=1, platform='linux', target=target, execution='native', tools=tools, toolFiles=files,
        nativeDependencies=list(native_paths), nativeFiles={name: identity(toolkit, payload / path)
            for name, path in native_paths.items()}, frontend=frontend, cargo=cargo, payloadChecks=checks)


def seal(toolkit, target, partial):
    check_layout(toolkit)
    attempt = str(time.time_ns())
    final_manifest = toolkit / ('toolkit.partial.' + attempt + '.json' if partial else 'toolkit.json')
    archives = plain_directory(toolkit, 'archives', create=True)
    archive = archives / ('linux-dependencies.partial.' + attempt + '.tar.gz' if partial else 'linux-toolkit.tar.gz')
    if final_manifest.exists() or final_manifest.is_symlink() or archive.exists() or archive.is_symlink():
        raise ValueError('refusing to overwrite toolkit evidence; use a fresh toolkit destination')
    manifest = manifest_for(toolkit, target)
    missing = [name for name in ('node', 'npm', 'rustc', 'cargo') if name not in manifest['tools']]
    if missing and not partial:
        raise ValueError('missing toolkit components: ' + ', '.join(missing))
    failures = [check for check in manifest['payloadChecks'] if check['status'] != 'passed']
    if failures and not partial:
        raise ValueError('staged toolkit payload is invalid: ' + json.dumps(failures))
    with tempfile.NamedTemporaryFile(dir=archives, delete=False) as output:
        temporary = Path(output.name)
    pending_manifest = toolkit / ('toolkit.pending.' + attempt + '.json')
    try:
        with tarfile.open(temporary, 'w:gz', compresslevel=1) as bundle:
            bundle.add(toolkit / 'payload', arcname='payload', recursive=True)
        manifest['archive'] = dict(path=temporary.relative_to(toolkit).as_posix(), sha256=sha256(temporary))
        _check_toolkit_archive(toolkit, temporary, set())
        if not partial:
            atomic_json(pending_manifest, manifest)
            validate_toolkit(toolkit, pending_manifest, target=target, checkout=ROOT)
        temporary.replace(archive)
        manifest['archive']['path'] = archive.relative_to(toolkit).as_posix()
    finally:
        temporary.unlink(missing_ok=True)
        pending_manifest.unlink(missing_ok=True)
    if partial:
        manifest['readiness'] = dict(status='blocked', passed=False, requiredRust='1.91.0',
            missingComponents=missing, failedPayloadChecks=failures,
            reason='Dependency payload is incomplete; it is not an accepted toolkit or prepared application runtime.')
        atomic_json(final_manifest, manifest)
        return final_manifest, manifest['readiness']
    atomic_json(final_manifest, manifest)
    return final_manifest, dict(status='passed', passed=True, scope='toolkit integrity and prerequisites only')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True, help='Staged toolkit root containing payload/')
    parser.add_argument('--target', choices=(TARGET,), default=TARGET)
    parser.add_argument('--partial', action='store_true', help='Seal incomplete dependencies with readiness blocked')
    parser.add_argument('--download-rust', action='store_true', help='Fetch official checksum-bound Rust components; requires network access')
    parser.add_argument('--rust-version', default='1.91.0', type=checked_rust_version)
    args = parser.parse_args()
    toolkit = args.root.absolute()
    if sys.platform != 'linux' or os.environ.get('AIN_BUILD_STORAGE_PROFILE') != 'portable-linux':
        parser.error('run through python3 scripts/build-storage.py --profile portable-linux run ... on Linux')
    if not toolkit.is_dir() or not (toolkit / 'payload').is_dir():
        parser.error('the toolkit payload must already be staged according to README-linux-toolkit.md')
    report = dict(schema=1, status='running', passed=False, startedAt=time.time(), applicationBuildsRun=False)
    report_path = toolkit / f'prepare-result-{time.time_ns()}.json'
    try:
        if args.download_rust:
            complete_rust(toolkit, args.rust_version, args.target)
        path, readiness = seal(toolkit, args.target, args.partial)
        report.update(readiness, manifest=str(path))
        return 0 if report['passed'] else 2
    except (OSError, ValueError, subprocess.SubprocessError, tarfile.TarError) as error:
        report.update(status='blocked', passed=False, error=f'{type(error).__name__}: {error}')
        return 2
    finally:
        report['finishedAt'] = time.time()
        atomic_json(report_path, report)
        print(json.dumps(report, indent=2), flush=True)


if __name__ == '__main__':
    raise SystemExit(main())
