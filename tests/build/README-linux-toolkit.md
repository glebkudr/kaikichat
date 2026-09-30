# Offline Linux toolkit contract

This describes an unpackable native toolkit, not a supplied binary bundle. No
installation, download, extraction or Docker operation is performed by doctor
or the validator. Prepare the payload separately for the required Linux target
and distribution/ABI; preserve its actual versions and hashes in a reviewed
manifest. Node must be 26+, Rust and Cargo 1.91+ (the workspace minimum).
The checkout supplies `[patch.crates-io]` overrides in `vendor/sysinfo` and
`vendor/commonware-utils` for the locked `commonware-runtime 2026.9.0` graph.
Keep these reviewed source patches and the current Cargo.lock together; an
unpatched `sysinfo 0.39.6` registry dependency has a higher Rust requirement.

The standard layout is:

```text
toolkit/
  toolkit.json
  archives/linux-toolkit.tar.gz
  payload/
    bin/                         # executable node, npm, rustc, cargo and their launchers
    lib/                         # Node modules and toolchain runtime files
    Cargo.lock                   # exact lockfile from the selected checkout
    vendor/<crate>/               # Cargo.toml, .cargo-checksum.json, complete crate files
    cargo/config.toml
    cargo/registry/               # optional additional offline caches
    frontend/package-lock.json   # exact apps/desktop/package-lock.json from the checkout
    frontend/node_modules/       # complete locked Linux dependency installation, including .bin
    native/                      # native compiler/SDK, headers, libraries, pkg-config metadata
```

The archive contains `payload/...` with canonical modes (normally 0644 files,
0755 executables/directories). Every archive member must exist in the unpacked
layout with the same contents, lstat type and mode, including directories and
links. Each archived top-level subtree must contain exactly those members and
their parent directories: extra files, symlinks and even empty directories in
`payload` are rejected. Small archives may omit explicit directory entries;
their implicit parents must still be real directories. Validation does not follow
directory symlinks while checking this inventory. Mutable caches, archives and
the manifest alongside `payload` are outside that subtree and are not scanned.
Relative links must resolve to suitable members of this same archive;
leftovers from another extraction cannot satisfy them. Hardlinks must retain
the target inode, not merely identical bytes. Absolute paths, traversal, cycles,
duplicates, linked parent directories and special files are rejected. Vendor
directories use regular files and Cargo checksum metadata.

Before unpacking, compare the archive's SHA256 with the reviewed manifest and
use an extractor with path/link filtering, such as Python's `tarfile.data_filter`.
Then run the full validation below before executing anything from the payload.
Checksums establish consistency with that manifest; they are not a signature
from an external supplier.

## Manifest schema 1

All paths are relative to the toolkit directory and must resolve inside it.
All SHA256 values are lowercase 64-digit hexadecimal hashes of file bytes.
The following is a template; replace every `SHA256` and the crate inventory with
actual values. The validator rejects placeholders and incomplete payloads.

```json
{
  "schema": 1,
  "platform": "linux",
  "target": "x86_64-unknown-linux-gnu",
  "execution": "native",
  "archive": {"path": "archives/linux-toolkit.tar.gz", "sha256": "SHA256"},
  "tools": {"node": "26.0.0", "npm": "11.0.0", "rustc": "1.91.0", "cargo": "1.91.0"},
  "toolFiles": {
    "node": {"path": "payload/bin/node", "sha256": "SHA256"},
    "npm": {"path": "payload/bin/npm", "sha256": "SHA256"},
    "rustc": {"path": "payload/bin/rustc", "sha256": "SHA256"},
    "cargo": {"path": "payload/bin/cargo", "sha256": "SHA256"}
  },
  "nativeDependencies": ["cc", "pkg-config", "openssl", "webkit2gtk-4.1", "gtk+-3.0"],
  "nativeFiles": {
    "cc": {"path": "payload/native/bin/cc", "sha256": "SHA256"},
    "pkg-config": {"path": "payload/native/bin/pkg-config", "sha256": "SHA256"},
    "openssl": {"path": "payload/native/lib/pkgconfig/openssl.pc", "sha256": "SHA256"},
    "webkit2gtk-4.1": {"path": "payload/native/lib/pkgconfig/webkit2gtk-4.1.pc", "sha256": "SHA256"},
    "gtk+-3.0": {"path": "payload/native/lib/pkgconfig/gtk+-3.0.pc", "sha256": "SHA256"}
  },
  "frontend": {
    "packageLockPath": "payload/frontend/package-lock.json",
    "packageLockSha256": "SHA256",
    "nodeModulesPath": "payload/frontend/node_modules",
    "nodeModulesFiles": {
      "vitest/package.json": "SHA256",
      "vitest/vitest.mjs": "SHA256",
      "jsdom/package.json": "SHA256",
      "jsdom/lib/api.js": "SHA256",
      "typescript/package.json": "SHA256",
      "typescript/bin/tsc": "SHA256",
      "vite/package.json": "SHA256",
      "vite/bin/vite.js": "SHA256",
      "@tauri-apps/cli/package.json": "SHA256",
      "@tauri-apps/cli/tauri.js": "SHA256",
      ".bin/vitest": "SHA256",
      ".bin/tsc": "SHA256",
      ".bin/vite": "SHA256",
      ".bin/tauri": "SHA256"
    }
  },
  "cargo": {
    "locked": true,
    "offline": true,
    "lockPath": "payload/Cargo.lock",
    "lockSha256": "SHA256",
    "vendorPath": "payload/vendor",
    "vendorFiles": {
      "example-1.0.0/Cargo.toml": "SHA256",
      "example-1.0.0/src/lib.rs": "SHA256",
      "example-1.0.0/.cargo-checksum.json": "SHA256"
    },
    "configPath": "payload/cargo/config.toml",
    "configSha256": "SHA256"
  }
}
```

`aarch64-unknown-linux-gnu` is also an accepted target. The manifest target must
match the explicit validation request. A Docker-only payload is unsupported.
Record additional native prerequisites and their representative bound files in
both native maps. The full headers/libraries, transitive GTK dependencies, npm
payload and any extra caches must also be present in the checksum-bound archive;
a dependency label alone does not provision them. npm and GTK 3 are mandatory
in this full toolkit. Python-only suites still need neither Node nor Rust.
Foundry is a separate optional provision for EVM suites.

`nodeModulesFiles` is the complete installed-file inventory relative to
`nodeModulesPath`, not just the illustrative entries above. Include every package,
its metadata and its runtime files. Internal file symlinks such as `.bin` have
the target file's byte hash in this inventory; the archive additionally binds
the link target/type/mode. Directory/workspace links are unsupported: provision
an unpacked installation for this checkout. The validator matches installed
package names/versions to lockfile v2/v3 entries, requires declared main/module/
types/bin/export entry files and executable npm launchers, and requires the frontend
unit and desktop-build entrypoints. Optional packages for another OS/CPU/libc
may be absent; matching Linux GNU packages must be present. An npm download cache
alone does not meet this unpacked-payload contract.

Three exact upstream publication omissions are recognized. The official
[`@babel/helper-validator-identifier` 7.29.7 tarball](https://registry.npmjs.org/@babel/helper-validator-identifier/-/helper-validator-identifier-7.29.7.tgz)
declares `exports["."].types = "./lib/index.d.ts"` but does not contain that file.
The validator permits this absent declaration only when both the
lockfile integrity and the original package.json bytes match these identities:

```text
tarball SHA512 (package-lock integrity):
sha512-qehxGkRj55h/ff8EMaJ+cYhyaKlHIxqYDn682wQD7RNp9UujOQsHog2uS0r2vzr4pW+sXf90NeeayjcNaX3fFg==
published package.json SHA256:
1ad6aeced8b186ac259da45fea50ab9d65d3d958f6458c80e3c8013649d1b12d
```

The other two official archives contain their compiled distribution but declare
unpublished source exports:

| Package | Exact export route → absent target | Published package.json SHA256 |
|---|---|---|
| [`@standard-schema/spec` 1.1.0](https://registry.npmjs.org/@standard-schema/spec/-/spec-1.1.0.tgz) | `exports["."]["standard-schema-spec"]` → `./src/index.ts` | `58e5bd75ddd0684c88b07cd799585cc72a37fd6efb8ab0c936d49e230d5164fb` |
| [`vitest` 4.1.11](https://registry.npmjs.org/vitest/-/vitest-4.1.11.tgz) | `exports["./src/*"]` → `./src/*` | `a28126d97bcaf567da5bed69443b7f3bcd9a7a8c38c8b66e554686b6bb2c10e0` |

Their exact lockfile SHA512 values are pinned alongside these metadata hashes
in `_published_missing_frontend_export`; changing either rejects the omission.
Source fixtures in `tests/build/fixtures/` preserve all three metadata files
byte for byte. No declaration or source placeholder is generated. No other
package/version, export route or runtime entry is exempt. Legacy folder exports,
including Babel runtime's `"./regenerator/": "./regenerator/"`, require a real
contained directory with at least one checksum-bound file, with trailing slashes
on both sides. A file export cannot use this directory rule.

The complete installed-file inventory, each file's
SHA256, the checkout lock match, and archive contents/types/modes remain mandatory;
recomputing an inventory cannot make modified package metadata qualify. This
correction describes publication completeness, not a frontend test result.
Remove each exception when the lock moves to a corrected publication,
then validate the replacement payload and rerun Vitest and TypeScript.

`vendorFiles` is the complete regular-file inventory beneath `vendorPath`, with
paths relative to that directory. Each crate's `.cargo-checksum.json` is verified;
registry/git packages named in the bound lockfile must have matching vendored
name/version and package checksums. Extra or missing vendor files, changed locks,
and changed configuration are rejected. Additional registry caches do not replace
the complete vendor graph.

`payload/cargo/config.toml` is standalone (no `include`), selects that vendor
graph and disables Cargo network access. Relative source directories follow
Cargo's config-directory convention:

```toml
[source.crates-io]
replace-with = "vendored-sources"
[source.vendored-sources]
directory = "vendor"
[net]
offline = true
```

## Validation and use

From the project checkout, after safe unpacking:

```sh
python3 scripts/build-storage.py --profile portable-linux run python3 scripts/build_evidence.py toolkit-validate --root /opt/ain-toolkit --manifest /opt/ain-toolkit/toolkit.json --target x86_64-unknown-linux-gnu
```

The CLI also matches both payload locks against this checkout's `Cargo.lock` and
`apps/desktop/package-lock.json`; `--checkout` can select another checkout
explicitly. Validation hashes all archive members and bound inputs without
launching supplied tools. It is not a build or native acceptance result.

Use the unpacked toolchain without modifying global tool stores, and keep both
locked and offline Cargo switches on actual build commands:

```sh
toolkit_root=/opt/ain-toolkit
env PATH="$toolkit_root/payload/bin:$toolkit_root/payload/native/bin:$PATH" CARGO_HOME="$toolkit_root/payload/cargo" CARGO_NET_OFFLINE=true PKG_CONFIG_PATH="$toolkit_root/payload/native/lib/pkgconfig" python3 scripts/build-storage.py --profile portable-linux doctor --suite native-build
env PATH="$toolkit_root/payload/bin:$toolkit_root/payload/native/bin:$PATH" CARGO_HOME="$toolkit_root/payload/cargo" CARGO_NET_OFFLINE=true PKG_CONFIG_PATH="$toolkit_root/payload/native/lib/pkgconfig" python3 scripts/build-storage.py --profile portable-linux --output output/offline-build-001 run cargo build --locked --offline -p agentic-node
```

Relocatable native SDKs may additionally need their documented sysroot/linker
settings. For frontend suites, provision a real checkout-local
`apps/desktop/node_modules` directory from the validated payload, preserving its
files and internal `.bin` links, before doctor/build; do not fetch packages on the
offline host. Doctor checks the selected suite's actual tools and libraries; it does
not install missing dependencies. Runtime suites need only their own
dependencies, such as local Unix IPC for native-runtime. The macOS bundle
producer remains a macOS-only gate.
