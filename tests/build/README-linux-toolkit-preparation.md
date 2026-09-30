# Completing a staged portable Linux toolkit

`scripts/prepare-linux-toolkit.py` seals the payload layout described in
`README-linux-toolkit.md`. It does not install operating-system packages or build
the application. The staged native SDK currently supports Ubuntu 24.04 / glibc
2.39 on x86_64. This preparer rejects another target rather than relabeling these
native binaries.

Run it from the selected checkout through the portable wrapper. For an incomplete
payload, this creates a new archive and a versioned partial manifest with
`readiness.status = blocked` and `passed = false`; exit code 2 is intentional:

```sh
python3 scripts/build-storage.py --profile portable-linux run python3 scripts/prepare-linux-toolkit.py --root .local/toolchains/toolkit --partial
```

For a staged payload missing Rust, the following explicitly downloads official
Rust 1.91.0 components when the host's network policy allows it. The preparer
checks the official channel-manifest SHA256 and every component archive SHA256,
rejects unsafe extraction paths, installs into a temporary local prefix, verifies
installed bytes, and includes Rust provenance inside the final payload archive:

```sh
python3 scripts/build-storage.py --profile portable-linux run python3 scripts/prepare-linux-toolkit.py --root .local/toolchains/toolkit --download-rust --rust-version 1.91.0
```

This command still rejects an incomplete frontend payload or any other failed
toolkit validation. Existing accepted manifests and archives are preserved;
prepare another destination to replace an already accepted toolkit. A network
denial is a blocked provisioning attempt, not an application test failure.

The September 16 dependency package was sealed as incomplete: it contains no
Rust compiler, and its original validation rejected Babel's unpublished type
declaration. The current checkout's reviewed source patches lower the required
Rust version to 1.91.0. Complete that compiler before sealing a new toolkit, and
refresh the payload Cargo.lock to the current checkout; the partial package's
old lock does not describe the patched graph. See the exact Babel publication
exception in `README-linux-toolkit.md`; do not fabricate the missing declaration.
Neither change upgrades the historical partial manifest to an accepted toolkit.

After verifying the distributed archive SHA256 against its partial manifest,
extract it with Python's `tarfile.data_filter` into a fresh toolkit directory.
The manifest and `payload/` must share that directory. This archive is a dependency
checkpoint and does not supply prepared application binaries.

The supplied `payload/activate.sh` computes paths relative to itself. From this
checkout's actual staging location:

```sh
source .local/toolchains/toolkit/payload/activate.sh
python3 scripts/build-storage.py --profile portable-linux doctor --suite desktop-build
```

The actual SDK pkg-config directories are
`payload/native/usr/lib/x86_64-linux-gnu/pkgconfig` and
`payload/native/usr/share/pkgconfig`; the activation script exports both.
It also selects the local GCC sysroot wrappers, compiler libraries, Node/npm,
Foundry/solc, standalone Cargo configuration and vendored crate graph. The
immutable payload retains its reference `cargo/config.toml`; writable Cargo
state belongs in the toolkit's sibling `cache/cargo` directory. Activation
creates its vendor configuration only when absent and rejects different
existing bytes. Cargo must not write `.package-cache` or its global cache
database inside the checksum-bound payload.

Once the full preparer reports success, independently validate its output:

```sh
python3 scripts/build-storage.py --profile portable-linux run python3 scripts/build_evidence.py toolkit-validate --root .local/toolchains/toolkit --manifest .local/toolchains/toolkit/toolkit.json --target x86_64-unknown-linux-gnu
```

Only then build the application with the toolkit. Toolkit integrity or doctor
PASS alone is not product acceptance.
