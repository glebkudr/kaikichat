#!/usr/bin/env python3
"""Small shared command log, desktop build provenance and suite preflight."""
import argparse
import errno
import hashlib
import json
import os
from pathlib import Path
import platform
import posixpath
import re
import shutil
import signal
import socket
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
from datetime import datetime, timedelta, timezone
import uuid


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = 1
FRONTEND_FILES = dict(vitest='vitest/vitest.mjs', jsdom='jsdom/package.json',
    tauri='@tauri-apps/cli/tauri.js', typescript='typescript/bin/tsc', vite='vite/bin/vite.js')


def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + '.tmp')
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')
    temporary.replace(path)


def new_run_dir(base):
    name = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ') + '-' + uuid.uuid4().hex[:12]
    output = Path(base) / name
    output.mkdir(parents=True, exist_ok=False)
    return output


def _stop_group(process):
    cleanup = dict(processGroup=os.name == 'posix', signals=[])
    if os.name == 'posix':
        # The direct process can exit while descendants remain. Do not condition
        # group cleanup on poll()/wait() of that process alone.
        for sig in (signal.SIGTERM, signal.SIGKILL):
            try:
                os.killpg(process.pid, sig)
                cleanup['signals'].append(sig.name)
            except ProcessLookupError:
                break
            if sig == signal.SIGTERM:
                time.sleep(0.25)
    else:
        # taskkill follows only this command's owned descendant tree.
        result = subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
            capture_output=True, timeout=5)
        cleanup.update(taskkillExitCode=result.returncode)
        if process.poll() is None:
            process.kill()
    process.wait(timeout=5)
    return cleanup


def run_command(name, args, report, *, output, cwd, env=None, timeout=None):
    output, cwd = Path(output), Path(cwd)
    output.mkdir(parents=True, exist_ok=True)
    if Path(name).name != name:
        raise ValueError('command name must be a filename component')
    log_path = output / (name + '.log')
    # Names are stage identities: accidentally executing one twice must not
    # overwrite the earlier compiler evidence in this attempt.
    if log_path.exists():
        raise ValueError(f'command log already exists: {log_path}')
    args = [str(value) for value in args]
    started_at = datetime.now(timezone.utc)
    entry = dict(name=name, args=args, cwd=str(cwd), logPath=str(log_path),
        status='running', startedAt=started_at.isoformat(),
        deadlineAt=(started_at + timedelta(seconds=timeout)).isoformat() if timeout is not None else None,
        timeoutSeconds=timeout, elapsedSeconds=0, exitCode=None)
    report.setdefault('commands', []).append(entry)
    write_json(output / 'check.json', report)
    started, process = time.monotonic(), None
    try:
        with log_path.open('w') as log:
            process = subprocess.Popen(args, cwd=cwd, env=env, stdout=log,
                stderr=subprocess.STDOUT, start_new_session=os.name == 'posix')
            entry['processId'] = process.pid
            write_json(output / 'check.json', report)
            code = process.wait(timeout=timeout)
            entry.update(exitCode=code, status='succeeded' if code == 0 else 'failed')
            if code:
                raise subprocess.CalledProcessError(code, args)
        return log_path.read_text(errors='replace')
    except BaseException as error:
        entry['error'] = f'{type(error).__name__}: {error}'
        if isinstance(error, subprocess.TimeoutExpired):
            entry['status'] = 'timed_out'
        elif process is None:
            entry['status'] = 'spawn_error'
        elif not isinstance(error, subprocess.CalledProcessError):
            entry['status'] = 'interrupted'
        if process is not None and not isinstance(error, subprocess.CalledProcessError):
            try:
                entry['cleanup'] = _stop_group(process)
                entry['exitCode'] = process.returncode
            except BaseException as cleanup_error:
                entry['cleanupError'] = repr(cleanup_error)
        raise
    finally:
        entry['elapsedSeconds'] = round(time.monotonic() - started, 6)
        write_json(output / 'check.json', report)


def _digest_path(path, ancestors=()):
    path = Path(path)
    mode = stat.S_IMODE(path.lstat().st_mode)
    if path.is_symlink():
        resolved = path.resolve(strict=True)
        if resolved in ancestors:
            raise ValueError(f'cyclic artifact/source link: {path}')
        value = ['link', mode, os.readlink(path), _digest_path(resolved, (*ancestors, resolved))]
    elif path.is_dir():
        value = ['directory', mode, [[p.name, _digest_path(p, ancestors)]
            for p in sorted(path.iterdir())]]
    elif path.is_file():
        hasher = hashlib.sha256()
        with path.open('rb') as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b''):
                hasher.update(chunk)
        value = ['file', mode, hasher.hexdigest()]
    else:
        raise ValueError(f'unsupported artifact/source type: {path}')
    return hashlib.sha256(json.dumps(value, separators=(',', ':')).encode()).hexdigest()


def _file_sha(path):
    result = hashlib.sha256()
    with Path(path).open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            result.update(chunk)
    return result.hexdigest()


def source_snapshot(root):
    root = Path(root)
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    raw = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=root)
    files = {}
    for name in sorted(set(os.fsdecode(p) for p in raw.split(b'\0') if p)):
        path = root / name
        files[name] = _digest_path(path) if path.exists() or path.is_symlink() else None
    return dict(head=head, files=files)


def _paths(artifacts):
    return {name: str(Path(path).absolute()) for name, path in sorted(artifacts.items())}


def _artifacts(paths):
    try:
        return {name: dict(path=path, resolvedPath=str(Path(path).resolve(strict=True)),
            sha256=_digest_path(path)) for name, path in paths.items()}
    except (OSError, RuntimeError) as error:
        raise ValueError(f'artifact unavailable: {error}') from error


def begin_build(root, configuration, artifacts):
    return dict(schema=SCHEMA, status='building', source=source_snapshot(root),
        configuration=configuration, artifactPaths=_paths(artifacts))


def seal_build(root, draft, path):
    if draft.get('schema') != SCHEMA or draft.get('status') != 'building':
        raise ValueError('invalid building manifest status/schema')
    if source_snapshot(root) != draft['source']:
        raise ValueError('source changed during build; candidate was not prepared')
    manifest = dict(draft, status='prepared', artifacts=_artifacts(draft['artifactPaths']))
    write_json(path, manifest)
    return manifest


def host_target():
    machine = {'arm64': 'aarch64', 'AMD64': 'x86_64'}.get(platform.machine(), platform.machine())
    suffix = {'darwin': 'apple-darwin', 'linux': 'unknown-linux-gnu', 'win32': 'pc-windows-msvc'}.get(sys.platform)
    if suffix is None or machine not in {'aarch64', 'x86_64'}:
        raise ValueError(f'unsupported host target: {sys.platform}/{machine}')
    return f'{machine}-{suffix}'


def _cargo_config_layers(path, identities, ancestors=(), optional=False):
    """Cargo include order, with each value retaining its defining file's base."""
    path = Path(os.path.abspath(path))
    try:
        if not path.exists() and not path.is_symlink() and optional:
            identities[str(path)] = None
            return []
        canonical = path.resolve(strict=True)
        if canonical in ancestors:
            raise ValueError(f'cyclic Cargo config include: {path}')
        if not path.is_file():
            raise ValueError(f'Cargo config include is not a file: {path}')
        parsed = tomllib.loads(path.read_text())
        identities[str(path)] = _digest_path(path)
        includes = parsed.get('include', [])
        if not isinstance(includes, list):
            raise ValueError(f'Cargo config include must be an array: {path}')
        layers = []
        for entry in includes:
            if isinstance(entry, str):
                include, optional_include = entry, False
            elif (isinstance(entry, dict) and set(entry) <= {'path', 'optional'}
                    and isinstance(entry.get('optional', False), bool)):
                include, optional_include = entry.get('path'), entry.get('optional', False)
            else:
                raise ValueError(f'invalid Cargo config include entry: {path}')
            if not isinstance(include, str) or not include or Path(include).suffix != '.toml':
                raise ValueError(f'Cargo config include requires a .toml path: {path}')
            included = Path(include)
            if not included.is_absolute():
                included = path.parent / included
            layers.extend(_cargo_config_layers(included, identities, (*ancestors, canonical), optional_include))
        return [*layers, (path, parsed)]
    except (OSError, RuntimeError, tomllib.TOMLDecodeError) as error:
        raise ValueError(f'Cargo config/include unavailable or invalid: {path}: {error}') from error


def cargo_inputs(root, kind, env):
    """Read build inputs without locating or invoking any compiler/manager."""
    root = Path(root).absolute()
    target = Path(env.get('CARGO_TARGET_DIR', str(root / 'target')))
    target = target if target.is_absolute() else root / target
    if target.resolve() != (root / 'target').resolve():
        raise ValueError('configuration CARGO_TARGET_DIR must resolve to this checkout target; refusing stale artifact paths')
    for selector in ('CARGO_BUILD_TARGET', 'CARGO_BUILD_TARGET_DIR'):
        if selector in env:
            raise ValueError(f'configuration {selector} is unsupported by the host artifact paths')
    working = root / 'apps/desktop/src-tauri' if kind == 'desktop' else root
    config_dirs = [directory / '.cargo' for directory in [working, *working.parents]]
    config_dirs.append(Path(env.get('CARGO_HOME', str(Path(env.get('HOME', str(Path.home()))) / '.cargo'))))
    configs, vendors, build, sources = {}, {}, {}, {}
    # Cargo home/outer configs have lower priority; includes merge left-to-right
    # before their including file. Only path-bearing build/source settings need
    # interpretation here; the complete file bytes bind all other settings.
    for directory in reversed(list(dict.fromkeys(config_dirs))):
        path = directory / 'config'
        if not path.exists() and not path.is_symlink():
            path = directory / 'config.toml'
        if not path.exists() and not path.is_symlink():
            continue
        for origin, parsed in _cargo_config_layers(path, configs):
            settings, definitions = parsed.get('build', {}), parsed.get('source', {})
            if not isinstance(settings, dict) or not isinstance(definitions, dict):
                raise ValueError(f'invalid Cargo build/source configuration: {origin}')
            build.update({name: (value, origin) for name, value in settings.items()})
            for name, source in definitions.items():
                if not isinstance(source, dict):
                    raise ValueError(f'invalid Cargo source configuration: {origin}')
                sources.setdefault(name, {}).update({key: (value, origin) for key, value in source.items()})
    if 'target' in build:
        _, origin = build['target']
        raise ValueError(f'configuration build.target in {origin} is unsupported by host artifact paths')
    if 'target-dir' in build and 'CARGO_TARGET_DIR' not in env:
        value, origin = build['target-dir']
        if not isinstance(value, str) or not value:
            raise ValueError(f'invalid configuration build.target-dir in {origin}')
        configured = Path(value)
        configured = configured if configured.is_absolute() else origin.parent.parent / configured
        if configured.resolve() != (root / 'target').resolve():
            raise ValueError(f'configuration build.target-dir in {origin} would select different artifacts')
    for source in sources.values():
        if 'directory' in source:
            value, origin = source['directory']
            if not isinstance(value, str) or not value:
                raise ValueError(f'invalid configuration vendor directory in {origin}')
            vendor = Path(value)
            vendor = vendor if vendor.is_absolute() else origin.parent.parent / vendor
            if not vendor.is_dir():
                raise ValueError(f'configuration vendor directory is unavailable: {vendor}')
            vendors[str(vendor.absolute())] = _digest_path(vendor)
    return dict(targetDir=str(target.resolve()), cargoConfigs=configs, cargoVendors=vendors)


def build_configuration(kind, profile, *, features=(), env=None, tools=('rustc', 'cargo', 'node')):
    env = dict(os.environ if env is None else env)
    inputs = cargo_inputs(ROOT, kind, env)
    prefixes = ('CARGO_', 'RUST', 'TAURI_', 'VITE_', 'npm_config_', 'NPM_CONFIG_',
        'SCCACHE_', 'CC_', 'CXX_', 'AR_', 'LD_', 'CFLAGS_',
        'CXXFLAGS_', 'CPPFLAGS_', 'LDFLAGS_', 'HOST_', 'TARGET_', 'PKG_CONFIG_')
    names = {'PATH', 'CC', 'CXX', 'AR', 'LD', 'CFLAGS', 'CXXFLAGS', 'CPPFLAGS', 'LDFLAGS',
        'SDKROOT', 'DEVELOPER_DIR', 'MACOSX_DEPLOYMENT_TARGET', 'NODE_OPTIONS',
        'LIBRARY_PATH', 'LD_LIBRARY_PATH'}
    # Environment can include registry credentials. Bind values without copying
    # them into reports; explicit profile/features/commands remain readable.
    environment = {key: hashlib.sha256(value.encode()).hexdigest() for key, value in sorted(env.items())
        if key in names or key.startswith(prefixes)}
    identities = {}
    tools = tools if isinstance(tools, dict) else {Path(name).name: name for name in tools}
    for name, executable in tools.items():
        resolved = shutil.which(str(executable), path=env.get('PATH'))
        if resolved is None:
            raise ValueError(f'build tool unavailable: {executable}')
        launcher = str(Path(resolved).resolve())
        actual = launcher
        rustup = shutil.which('rustup', path=env.get('PATH')) if name in {'cargo', 'rustc'} else None
        proxy = rustup is not None and (os.path.samefile(launcher, rustup)
            or _file_sha(launcher) == _file_sha(rustup))
        if name in {'cargo', 'rustc'} and (Path(launcher).name == 'rustup' or proxy):
            # Resolve the selected compiler without launching Cargo, Forge or a
            # build. Hashing only the shared rustup shim would miss tool changes.
            actual = subprocess.check_output([rustup or launcher, 'which', name], env=env,
                cwd=ROOT, text=True, stderr=subprocess.STDOUT, timeout=30).strip()
        identities[name] = dict(path=str(Path(actual).resolve()), sha256=_digest_path(actual),
            launcher=launcher, launcherSha256=_digest_path(launcher))
    return dict(kind=kind, profile=profile, features=sorted(features), env=environment,
        tools=identities, target=host_target(), **inputs)


def desktop_configuration(debug, e2e, env=None, node='node'):
    return build_configuration('desktop', 'debug' if debug else 'release',
        features=['e2e'] if e2e else [], env=env,
        tools=dict(rustc='rustc', cargo='cargo', node=node))


def desktop_artifacts(root, debug):
    return dict(bundle=Path(root) / 'target' / ('debug' if debug else 'release') /
        'bundle/macos/Kaiki Chat.app')


def _version(value):
    if not isinstance(value, str):
        raise ValueError('tool version must be a string')
    match = re.search(r'\b(\d+)\.(\d+)(?:\.(\d+))?', value.lstrip('v'))
    if match is None:
        raise ValueError(f'unrecognizable version: {value}')
    return tuple(int(part or 0) for part in match.groups())


def _unix_ipc_probe():
    """Exercise the daemon's local stream IPC capability without starting it."""
    item = dict(name='unix-ipc', status='blocked', operation='socket', errno=None,
        family='AF_UNIX', socketType='SOCK_STREAM', timeoutSeconds=1, operations=[])
    streams, directory = [], None

    def failure(error):
        return dict(errno=error.errno, errnoName=errno.errorcode.get(error.errno),
            errorType=type(error).__name__, reason=str(error))

    def step(operation, action, endpoint=None):
        observed = dict(operation=operation, endpoint=endpoint, status='passed', errno=None)
        item['operations'].append(observed)
        try:
            result = action()
        except OSError as error:
            observed.update(status='blocked', **failure(error))
            raise
        finally:
            item.update(observed)
        return result

    def stream():
        if not hasattr(socket, 'AF_UNIX'):
            raise OSError(errno.EAFNOSUPPORT, 'AF_UNIX is unavailable')
        connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        streams.append(connection)
        connection.settimeout(item['timeoutSeconds'])
        return connection

    try:
        listener = step('socket', stream, 'listener')
        directory = step('temporary_directory',
            lambda: tempfile.TemporaryDirectory(prefix='ain-ipc-'))
        path = str(Path(directory.name) / 'probe.sock')
        step('bind', lambda: listener.bind(path), 'listener')
        step('listen', lambda: listener.listen(1), 'listener')
        client = step('socket', stream, 'client')
        # A listening local socket needs no server thread: connect queues the
        # connection, then accept verifies it. Both waits have a finite timeout.
        step('connect', lambda: client.connect(path), 'client')
        accepted, _ = step('accept', listener.accept, 'listener')
        streams.append(accepted)
    except OSError:
        pass  # The exact failed operation and errno are already recorded.
    finally:
        cleanup = dict(status='passed', socketsClosed=0, temporaryDirectoryRemoved=True)
        errors = []
        for connection in reversed(streams):
            try:
                connection.close()
                cleanup['socketsClosed'] += 1
            except OSError as error:
                errors.append(dict(operation='close', **failure(error)))
        if directory is not None:
            try:
                directory.cleanup()
            except OSError as error:
                errors.append(dict(operation='remove_temporary_directory', **failure(error)))
            cleanup['temporaryDirectoryRemoved'] = not Path(directory.name).exists()
        if errors:
            cleanup.update(status='failed', errors=errors)
            item.update(errors[0])
            item.update(status='failed', operation='cleanup')
        item['cleanup'] = cleanup
    return item


def doctor(root, suite, *, profile='mac-apfs', phase='build', env=None):
    """Preflight only the selected suite. Passing is not a test/build result."""
    root, env = Path(root), dict(os.environ if env is None else env)
    python = {'python3': (3, 11, 0)}
    rust = {'cargo': (1, 91, 0), 'rustc': (1, 91, 0), 'cc': None, 'pkg-config': None}
    frontend = {'node': (26, 0, 0), 'npm': None}
    suites = {
        'build-tooling': dict(python, git=None),
        'frontend-unit': frontend, 'backend-unit': rust, 'native-build': rust,
        'desktop-build': dict(rust, **frontend),
        'evm': {'forge': None, 'anvil': None, 'cast': None},
        'native-runtime': dict(python, git=None),
        'native-macos': dict(python, git=None, codesign=None,
            **(dict(rust, **frontend) if phase == 'build' else {})),
    }
    report = dict(schema=SCHEMA, kind='preflight', suite=suite, profile=profile,
        phase=phase, status='unsupported', passed=False, requirements=[])
    if suite not in suites or phase not in {'build', 'runtime'}:
        report['reason'] = 'Unknown suite or phase; no fallback was selected.'
        return report
    if {'mac-apfs': 'darwin', 'portable-linux': 'linux'}.get(profile) != sys.platform:
        report['reason'] = 'Unsupported storage profile for this host.'
        return report
    if suite == 'native-runtime' and phase == 'build':
        report['reason'] = 'native-runtime has no build phase; use native-build for build preflight.'
        return report
    if suite == 'native-macos' and sys.platform != 'darwin':
        report['reason'] = 'The native macOS bundle gate requires macOS.'
        return report
    if suite == 'desktop-build' and sys.platform != 'darwin':
        # Linux dependency readiness is useful even though the current packaged
        # app producer is a macOS gate; it does not claim Linux installer support.
        report['scope'] = 'desktop dependencies; macOS bundle producer remains host-specific'
    for name, minimum in suites[suite].items():
        executable = env.get('AIN_NODE', 'node') if name == 'node' else name
        if name in {'forge', 'anvil', 'cast'} and env.get('AIN_FOUNDRY_BIN'):
            executable = str(Path(env['AIN_FOUNDRY_BIN']) / name)
        resolved = shutil.which(executable, path=env.get('PATH', ''))
        item = dict(name=name, status='blocked')
        report['requirements'].append(item)
        if resolved is None:
            item['reason'] = f'Required tool is unavailable: {executable}'
            continue
        item['path'] = resolved
        if name == 'codesign':
            item['status'] = 'passed'
            continue
        try:
            result = subprocess.run([resolved, '--version'], cwd=root,
                env=dict(env, RUSTUP_AUTO_INSTALL='0'),
                text=True, capture_output=True, timeout=10)
            version = (result.stdout or result.stderr).strip()
            if result.returncode:
                raise ValueError(f'{name} version probe exited {result.returncode}: {version}')
            if minimum is not None and _version(version) < minimum:
                raise ValueError(f'{name} requires >= {".".join(map(str, minimum))}; found {version}')
            item.update(status='passed', version=version)
        except (OSError, ValueError, subprocess.TimeoutExpired) as error:
            item.update(status='failed', reason=str(error))
    files = {}
    if suite == 'frontend-unit':
        files.update({name: 'apps/desktop/node_modules/' + FRONTEND_FILES[name] for name in ('vitest', 'jsdom')})
    if suite == 'desktop-build' or (suite == 'native-macos' and phase == 'build'):
        files.update({name: 'apps/desktop/node_modules/' + FRONTEND_FILES[name]
            for name in ('tauri', 'typescript', 'vite')})
    for name, relative in files.items():
        path = root / relative
        report['requirements'].append(dict(name=name, path=str(path),
            status='passed' if path.is_file() else 'blocked'))
    if suite == 'native-runtime':
        report['requirements'].append(_unix_ipc_probe())
    if suite in {'native-build', 'desktop-build'} and sys.platform == 'linux' and all(
            item['status'] == 'passed' for item in report['requirements']):
        libraries = ['openssl'] + (['webkit2gtk-4.1', 'gtk+-3.0'] if suite == 'desktop-build' else [])
        for library in libraries:
            try:
                result = subprocess.run(['pkg-config', '--exists', library], cwd=root,
                    env=env, capture_output=True, timeout=10)
                report['requirements'].append(dict(name=library,
                    status='passed' if result.returncode == 0 else 'blocked'))
            except (OSError, subprocess.TimeoutExpired) as error:
                report['requirements'].append(dict(name=library, status='failed', reason=str(error)))
    states = {item['status'] for item in report['requirements']}
    report['status'] = 'failed' if 'failed' in states else 'blocked' if 'blocked' in states else 'passed'
    report['passed'] = report['status'] == 'passed'
    return report


def _contained(root, relative):
    if not isinstance(relative, str) or not relative or '\\' in relative:
        raise ValueError('toolkit path must be a nonempty relative POSIX path')
    path = Path(relative)
    if path.is_absolute() or '..' in path.parts or str(path) == '.':
        raise ValueError(f'unsafe toolkit path: {relative}')
    result = Path(root) / path
    if not result.resolve().is_relative_to(Path(root).resolve()):
        raise ValueError(f'toolkit path/link escapes its root: {relative}')
    return result


def _check_sha(path, digest):
    if not isinstance(digest, str) or re.fullmatch('[0-9a-f]{64}', digest) is None:
        raise ValueError(f'invalid sha256 for {path}')
    if not path.is_file() or _file_sha(path) != digest:
        raise ValueError(f'file unavailable or checksum mismatch: {path}')


def _check_inventory(directory, files, *, file_links=False):
    if directory.is_symlink() or not directory.is_dir() or not isinstance(files, dict) or not files:
        raise ValueError(f'toolkit file inventory is unavailable: {directory}')
    actual = set()
    for item in directory.rglob('*'):
        if item.is_symlink():
            if not file_links or not item.is_file():
                raise ValueError(f'unsupported toolkit inventory link: {item}')
            _contained(directory, item.relative_to(directory).as_posix())
        elif item.is_dir():
            continue
        elif not item.is_file():
            raise ValueError(f'unsupported toolkit inventory file type: {item}')
        actual.add(item.relative_to(directory).as_posix())
    if actual != set(files):
        raise ValueError(f'toolkit file inventory does not match manifest: {directory}')
    for relative, digest in files.items():
        _check_sha(_contained(directory, relative), digest)


def _published_missing_frontend_export(name, entry, metadata_path, route, value):
    # These integrity-matching official tarballs never contained the named
    # source/type exports. Only their exact published metadata qualifies;
    # installed files, the lock and the complete toolkit archive stay bound.
    # See tests/build/README-linux-toolkit.md for provenance and removal rule.
    published = {
        '@babel/helper-validator-identifier': ('7.29.7',
            'sha512-qehxGkRj55h/ff8EMaJ+cYhyaKlHIxqYDn682wQD7RNp9UujOQsHog2uS0r2vzr4pW+sXf90NeeayjcNaX3fFg==',
            ('.', 'types'), './lib/index.d.ts',
            '1ad6aeced8b186ac259da45fea50ab9d65d3d958f6458c80e3c8013649d1b12d'),
        '@standard-schema/spec': ('1.1.0',
            'sha512-l2aFy5jALhniG5HgqrD6jXLi/rUWrKvqN/qJx6yoJsgKhblVd+iqqU4RCXavm/jPityDo5TCvKMnpjKnOriy0w==',
            ('.', 'standard-schema-spec'), './src/index.ts',
            '58e5bd75ddd0684c88b07cd799585cc72a37fd6efb8ab0c936d49e230d5164fb'),
        'vitest': ('4.1.11',
            'sha512-fhACrNXUidIbGSBr5FlbuBkO7VWC1ZyLl0DO4CU2DrQoAPxX84Ysxs+HeGQpii5lZWV1Q4gBZTTu49mF+A6Edw==',
            ('./src/*',), './src/*',
            'a28126d97bcaf567da5bed69443b7f3bcd9a7a8c38c8b66e554686b6bb2c10e0'),
    }
    expected = published.get(name)
    return expected is not None and (
        entry.get('version'), entry.get('integrity'), route, value,
        _file_sha(metadata_path)) == expected


def _check_frontend(root, frontend, target, checkout):
    lock = _contained(root, frontend['packageLockPath'])
    modules = _contained(root, frontend['nodeModulesPath'])
    _check_sha(lock, frontend['packageLockSha256'])
    if checkout is not None:
        _check_sha(Path(checkout) / 'apps/desktop/package-lock.json', frontend['packageLockSha256'])
    inventory = frontend['nodeModulesFiles']
    _check_inventory(modules, inventory, file_links=True)
    locked = json.loads(lock.read_text())
    packages = locked.get('packages') if isinstance(locked, dict) else None
    if (not isinstance(packages, dict) or not isinstance(packages.get(''), dict)
            or locked.get('lockfileVersion') not in (2, 3)):
        raise ValueError('toolkit requires a complete npm package-lock with package entries')
    for field in ('dependencies', 'devDependencies'):
        for name in packages[''].get(field, {}):
            if 'node_modules/' + name not in packages:
                raise ValueError(f'frontend dependency is not locked: {name}')
    for relative in FRONTEND_FILES.values():
        if relative not in inventory:
            raise ValueError(f'frontend suite entry is absent from the offline payload: {relative}')

    host = dict(os='linux', cpu='x64' if target.startswith('x86_64-') else 'arm64', libc='glibc')
    for location, entry in packages.items():
        if not location:
            continue
        if not location.startswith('node_modules/') or not isinstance(entry, dict) or entry.get('link'):
            raise ValueError(f'unsupported locked frontend package location: {location}')
        package_dir = _contained(modules, location.removeprefix('node_modules/'))
        metadata_path = package_dir / 'package.json'
        applicable = True
        for field, wanted in host.items():
            allowed = entry.get(field, [])
            if not isinstance(allowed, list) or any(not isinstance(value, str) for value in allowed):
                raise ValueError(f'invalid locked frontend platform constraint: {location}')
            positive = [value for value in allowed if not value.startswith('!')]
            applicable &= ('!' + wanted not in allowed and (not positive or wanted in positive or 'any' in positive))
        if not metadata_path.is_file() and not applicable and entry.get('optional') is True:
            continue
        metadata = json.loads(metadata_path.read_text())
        name = entry.get('name', location.removeprefix('node_modules/').rsplit('/node_modules/', 1)[-1])
        if (not isinstance(metadata, dict) or metadata.get('name') != name
                or not isinstance(entry.get('version'), str) or metadata.get('version') != entry['version']):
            raise ValueError(f'frontend package metadata differs from its lock: {location}')
        if metadata_path.relative_to(modules).as_posix() not in inventory:
            raise ValueError(f'frontend package metadata is not bound: {location}')
        entries = [metadata[key] for key in ('main', 'module', 'types', 'typings') if metadata.get(key)]
        bins = metadata.get('bin', {})
        bins = {name.rsplit('/', 1)[-1]: bins} if isinstance(bins, str) else bins
        if not isinstance(bins, dict):
            raise ValueError(f'invalid frontend package bin entries: {location}')
        entries.extend(bins.values())
        for value in entries:
            candidate = _contained(package_dir, value)
            # Node permits extensionless main paths and directory indexes.
            candidates = [candidate, Path(str(candidate) + '.js'), Path(str(candidate) + '.json'),
                Path(str(candidate) + '.node'), candidate / 'index.js', candidate / 'index.json']
            if not any(item.is_file() and item.relative_to(modules).as_posix() in inventory for item in candidates):
                raise ValueError(f'frontend package entry is missing: {location}: {value}')
        exports = [((), metadata['exports'])] if 'exports' in metadata else []
        while exports:
            route, value = exports.pop()
            if isinstance(value, dict):
                exports.extend(((*route, key), child) for key, child in value.items())
            elif isinstance(value, list):
                exports.extend(((*route, index), child) for index, child in enumerate(value))
            elif value is not None:
                if not isinstance(value, str) or not value.startswith('./'):
                    raise ValueError(f'invalid frontend package export: {location}')
                candidate = _contained(package_dir, value)
                candidates = package_dir.glob(value) if '*' in value else [candidate]
                # Published legacy folder mappings describe a prefix, not a
                # file. Require both explicit trailing slashes and at least
                # one contained, checksum-bound file in the mapped directory.
                if value.endswith('/') and any(isinstance(key, str) and key.startswith('./')
                        and key.endswith('/') for key in route) and candidate.is_dir():
                    candidates = candidate.rglob('*')
                if not any(item.is_file() and item.relative_to(modules).as_posix() in inventory for item in candidates):
                    if _published_missing_frontend_export(name, entry, metadata_path, route, value):
                        continue
                    raise ValueError(f'frontend package export is missing: {location}: {value}')
        for executable in bins:
            package_parent = package_dir.parent.parent if package_dir.parent.name.startswith('@') else package_dir.parent
            launcher = _contained(package_parent, '.bin/' + executable)
            if launcher.relative_to(modules).as_posix() not in inventory or not os.access(launcher, os.X_OK):
                raise ValueError(f'frontend npm launcher is unavailable: {location}: {executable}')


def _check_toolkit_archive(root, archive, required):
    with tarfile.open(archive, 'r:*') as bundle:
        members = {}
        for member in bundle:
            name = member.name.rstrip('/') if member.isdir() else member.name
            if not name or name != posixpath.normpath(name):
                raise ValueError(f'noncanonical toolkit archive member: {member.name}')
            _contained(root, name)
            if name in members:
                raise ValueError(f'duplicate toolkit archive member: {name}')
            if not (member.isdir() or member.isfile() or member.issym() or member.islnk()):
                raise ValueError(f'unsafe archive member type: {name}')
            members[name] = member
        if not required <= members.keys():
            raise ValueError('archive does not contain every bound toolkit input')

        def link_member(name, ancestors=()):
            if name in ancestors:
                raise ValueError(f'cyclic/self archive link: {name}')
            member = members.get(name)
            if member is None:
                raise ValueError(f'archive link target is not in this archive: {name}')
            if not (member.issym() or member.islnk()):
                return name
            if not member.linkname or member.linkname.startswith('/') or '\\' in member.linkname:
                raise ValueError(f'unsafe archive link: {name}')
            linked = posixpath.normpath(posixpath.join(posixpath.dirname(name), member.linkname)
                if member.issym() else member.linkname)
            _contained(root, linked)
            if member.islnk() and linked in members and (members[linked].issym() or members[linked].isdir()):
                raise ValueError(f'archive hardlink target is not a regular file: {name}')
            return link_member(linked, (*ancestors, name))

        for name, member in members.items():
            destination = _contained(root, name)
            for parent in destination.parents:
                if parent == root:
                    break
                if not stat.S_ISDIR(parent.lstat().st_mode):
                    raise ValueError(f'archive member parent is not a real directory: {name}')
            observed = destination.lstat()
            correct_type = (stat.S_ISDIR(observed.st_mode) if member.isdir() else
                stat.S_ISLNK(observed.st_mode) if member.issym() else stat.S_ISREG(observed.st_mode))
            if not correct_type or stat.S_IMODE(observed.st_mode) != stat.S_IMODE(member.mode):
                raise ValueError(f'archive member type/mode differs from unpacked toolkit: {name}')
            if member.issym() or member.islnk():
                linked = root / link_member(name)
                if member.issym():
                    if os.readlink(destination) != member.linkname or destination.resolve(strict=True) != linked.resolve(strict=True):
                        raise ValueError(f'archive symlink differs from unpacked toolkit: {name}')
                elif not os.path.samefile(destination, linked):
                    raise ValueError(f'archive hardlink inode differs from unpacked toolkit: {name}')
            elif member.isfile():
                digest = hashlib.sha256()
                with bundle.extractfile(member) as source:
                    for chunk in iter(lambda: source.read(1024 * 1024), b''):
                        digest.update(chunk)
                _check_sha(destination, digest.hexdigest())

        # Member hashes alone permit extra libraries or search directories in
        # the live payload. Close each archived top-level subtree over exactly
        # its members and their implicit parents (small archives need not emit
        # directory entries). Never scan the toolkit root: mutable caches and
        # the manifest/archive alongside payload are outside this boundary.
        expected = set()
        for name in members:
            parts = name.split('/')
            expected.update('/'.join(parts[:length]) for length in range(1, len(parts) + 1))
        pending = [root / name for name in sorted({name.split('/')[0] for name in members})
            if stat.S_ISDIR((root / name).lstat().st_mode)]
        while pending:
            directory = pending.pop()
            with os.scandir(directory) as entries:
                for entry in entries:
                    path = directory / entry.name
                    name = path.relative_to(root).as_posix()
                    if name not in expected:
                        raise ValueError(f'unpacked toolkit path is absent from its archive: {name}')
                    # lstat semantics also count dangling links as entries;
                    # directory links must not expand the archived topology.
                    if entry.is_dir(follow_symlinks=False):
                        pending.append(path)


def validate_toolkit(root, path, *, target, checkout=None):
    """Validate an unpacked, checksum-bound native Linux toolkit; never install it."""
    root = Path(root).resolve()
    try:
        manifest = json.loads(Path(path).read_text())
        if (not isinstance(manifest, dict) or manifest.get('schema') != 1 or manifest.get('platform') != 'linux'
                or manifest.get('execution') != 'native'
                or target not in {'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu'}
                or manifest.get('target') != target):
            raise ValueError('unsupported toolkit schema/platform/target/execution; native Linux is required')
        archive = _contained(root, manifest['archive']['path'])
        _check_sha(archive, manifest['archive']['sha256'])
        for name, minimum in dict(node=(26, 0, 0), npm=None, rustc=(1, 91, 0), cargo=(1, 91, 0)).items():
            version = _version(manifest['tools'][name])
            if minimum is not None and version < minimum:
                raise ValueError(f'toolkit requires {name} >= {minimum}')
            if name not in manifest['toolFiles']:
                raise ValueError(f'toolkit executable identity is unavailable: {name}')
        for name, identity in manifest['toolFiles'].items():
            binary = _contained(root, identity['path'])
            _check_sha(binary, identity['sha256'])
            if not os.access(binary, os.X_OK):
                raise ValueError(f'toolkit executable is not executable: {name}')
        native = set(manifest['nativeDependencies'])
        if not {'cc', 'pkg-config', 'openssl', 'webkit2gtk-4.1', 'gtk+-3.0'} <= native:
            raise ValueError('toolkit omits required native dependencies')
        for name in native:
            identity = manifest['nativeFiles'][name]
            _check_sha(_contained(root, identity['path']), identity['sha256'])
        frontend = manifest['frontend']
        _check_frontend(root, frontend, target, checkout)
        cargo = manifest['cargo']
        if cargo['locked'] is not True or cargo['offline'] is not True:
            raise ValueError('toolkit Cargo must be locked and offline')
        lock = _contained(root, cargo['lockPath'])
        config = _contained(root, cargo['configPath'])
        vendor = _contained(root, cargo['vendorPath'])
        _check_sha(lock, cargo['lockSha256'])
        if checkout is not None:
            _check_sha(Path(checkout) / 'Cargo.lock', cargo['lockSha256'])
        _check_sha(config, cargo['configSha256'])
        _check_inventory(vendor, cargo['vendorFiles'])
        parsed = tomllib.loads(config.read_text())
        replacement = parsed.get('source', {}).get('crates-io', {}).get('replace-with')
        directory = parsed.get('source', {}).get(replacement, {}).get('directory')
        if (parsed.get('include') or parsed.get('net', {}).get('offline') is not True or not isinstance(directory, str)
                or Path(directory).is_absolute()
                or (config.parent.parent / directory).resolve() != vendor.resolve()):
            raise ValueError('toolkit Cargo config must be standalone, select the bound vendor and disable network access')
        packages = {}
        for crate in vendor.iterdir():
            if not crate.is_dir():
                raise ValueError('vendor root must contain crate directories')
            package = tomllib.loads((crate / 'Cargo.toml').read_text())['package']
            key = (package['name'], package['version'])
            if key in packages:
                raise ValueError(f'duplicate vendored crate: {key}')
            checksum = json.loads((crate / '.cargo-checksum.json').read_text())
            if not isinstance(checksum.get('files'), dict) or not checksum['files']:
                raise ValueError(f'Cargo vendor checksum metadata is unavailable: {crate}')
            for relative, digest in checksum['files'].items():
                _check_sha(_contained(crate, relative), digest)
            packages[key] = checksum.get('package')
        for package in tomllib.loads(lock.read_text()).get('package', []):
            if package.get('source', '').startswith(('registry+', 'git+')):
                key = (package['name'], package['version'])
                if key not in packages or (package.get('checksum') is not None and packages[key] != package['checksum']):
                    raise ValueError(f'locked dependency is absent or changed in vendor: {key}')
        required = {cargo['lockPath'], cargo['configPath'], frontend['packageLockPath'],
            *(str(Path(cargo['vendorPath']) / name) for name in cargo['vendorFiles']),
            *(str(Path(frontend['nodeModulesPath']) / name) for name in frontend['nodeModulesFiles']),
            *(item['path'] for item in manifest['toolFiles'].values()),
            *(item['path'] for item in manifest['nativeFiles'].values())}
        required = {_contained(root, name).relative_to(root).as_posix() for name in required}
        _check_toolkit_archive(root, archive, required)
        return manifest
    except (OSError, KeyError, TypeError, RuntimeError, tarfile.TarError) as error:
        raise ValueError(f'toolkit manifest/archive/input unavailable: {error}') from error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='action', required=True)
    preflight = commands.add_parser('doctor')
    preflight.add_argument('--root', type=Path, default=ROOT)
    preflight.add_argument('--profile', choices=('mac-apfs', 'portable-linux'), default='mac-apfs')
    preflight.add_argument('--suite', required=True)
    preflight.add_argument('--phase', choices=('build', 'runtime'), default='build')
    preflight.add_argument('--output', type=Path)
    toolkit = commands.add_parser('toolkit-validate')
    toolkit.add_argument('--root', type=Path, required=True)
    toolkit.add_argument('--manifest', type=Path, required=True)
    toolkit.add_argument('--target', required=True)
    toolkit.add_argument('--checkout', type=Path, default=ROOT)
    begin = commands.add_parser('desktop-begin')
    begin.add_argument('--debug', action='store_true')
    begin.add_argument('--e2e', action='store_true')
    begin.add_argument('--output', type=Path)
    begin.add_argument('--node', default='node')
    stage = commands.add_parser('stage')
    stage.add_argument('--output', required=True, type=Path)
    stage.add_argument('--name', required=True)
    stage.add_argument('--cwd', type=Path, default=ROOT)
    stage.add_argument('--timeout', type=float, default=1800)
    stage.add_argument('command', nargs=argparse.REMAINDER)
    seal = commands.add_parser('seal')
    seal.add_argument('--output', type=Path, required=True)
    failure = commands.add_parser('failure')
    failure.add_argument('--output', type=Path, required=True)
    failure.add_argument('--stage', required=True)
    failure.add_argument('--error', required=True)
    args = parser.parse_args()
    if args.action == 'doctor':
        report = doctor(args.root, args.suite, profile=args.profile, phase=args.phase)
        if args.output:
            write_json(args.output / 'check.json', report)
        print(json.dumps(report, indent=2))
        return 0 if report['status'] == 'passed' else 2
    elif args.action == 'toolkit-validate':
        validate_toolkit(args.root, args.manifest, target=args.target, checkout=args.checkout)
        print(json.dumps(dict(status='passed', kind='toolkit-validation', target=args.target,
            manifest=str(args.manifest), buildPerformed=False)))
    elif args.action == 'desktop-begin':
        output = args.output or new_run_dir(ROOT / 'output/desktop-build')
        if (output / 'building.json').exists() or (output / 'check.json').exists():
            raise ValueError(f'build attempt already exists: {output}')
        draft = begin_build(ROOT, desktop_configuration(args.debug, args.e2e, node=args.node),
            desktop_artifacts(ROOT, args.debug))
        write_json(output / 'building.json', draft)
        write_json(output / 'check.json', dict(passed=False, commands=[]))
        print(json.dumps(dict(output=str(output))))
    elif args.action == 'stage':
        report = json.loads((args.output / 'check.json').read_text())
        command = args.command[1:] if args.command[:1] == ['--'] else args.command
        print(f'Build stage {args.name}: {args.output / (args.name + ".log")}', flush=True)
        run_command(args.name, command, report, output=args.output, cwd=args.cwd, timeout=args.timeout)
    elif args.action == 'failure':
        report = json.loads((args.output / 'check.json').read_text())
        report.update(passed=False, prepared=False, failure=dict(stage=args.stage, error=args.error))
        write_json(args.output / 'check.json', report)
    else:
        draft = json.loads((args.output / 'building.json').read_text())
        path = args.output / 'prepared-artifacts.json'
        seal_build(ROOT, draft, path)
        report = json.loads((args.output / 'check.json').read_text())
        report.update(prepared=True, preparedManifest=str(path), passed=False)
        write_json(args.output / 'check.json', report)
        print(f'Prepared build (runtime assertions not run): {path}', flush=True)


if __name__ == '__main__':
    # A supervising runner may cancel the helper while its command is in a new
    # session. Convert termination into the same owned-group cleanup path.
    def terminate(signum, frame):
        raise InterruptedError(f'build helper received signal {signum}')
    if os.name == 'posix':
        signal.signal(signal.SIGTERM, terminate)
    sys.exit(main())
