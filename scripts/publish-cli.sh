#!/bin/bash
# Publishes Kaiki Chat's builds at kaikichat.com/downloads, where
# kaikichat.com/install.sh and updates take them from. Each argument pair is
# a build and where it is:
#   scripts/publish-cli.sh macos-arm64 target/release linux-x86_64 DIR \
#       app-macos-arm64 "target/release/bundle/macos/Kaiki Chat.app"
# A command-line platform becomes kaiki-<platform>.tar.gz: a kaiki/
# directory with kaiki, agentic-node, agentic-cli, agentic-mcp and
# install.json, the marker that lets that install update itself. The app
# becomes kaiki-chat-<platform>.tar.gz, its bundle. Each is published twice,
# with its .sha256: at downloads/ (the latest, for install.sh) and at
# downloads/<version>/ (kept, for the release the network preset names).
# deployments/release.json then names the version and every build's URL and
# SHA-256; scripts/network-preset.py signs it into the preset. The version
# is the workspace's. Builds go to the server only, never into Git; they
# are copied into the chat-downloads volume of the Coolify application over
# SSH, as .env.prod names it.
set -euo pipefail
cd "$(dirname "$0")/.."
[ $# -ge 2 ] && [ $(($# % 2)) -eq 0 ] || { echo "usage: $0 BUILD PATH [BUILD PATH]..." >&2; exit 2; }
# .env.prod is local (gitignored): the main checkout's, or KAIKI_ENV_FILE.
set -a; . "${KAIKI_ENV_FILE:-./.env.prod}"; set +a
export COPYFILE_DISABLE=1
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
[ -n "$version" ] || { echo "no workspace version in Cargo.toml" >&2; exit 1; }
ssh_to() { ssh -i "${PROD_SSH_KEY/#\~/$HOME}" -p "$PROD_SSH_PORT" -o BatchMode=yes "$PROD_SSH_USER@$PROD_SSH_HOST" "$@"; }
volume="${COOLIFY_APP_UUID}_chat-downloads"
out=$(mktemp -d); trap 'rm -rf "$out"' EXIT
# No macOS extended attributes in the archives: Linux tar warns about them.
if tar --version 2>/dev/null | grep -q bsdtar; then pack=(tar --no-xattrs --no-mac-metadata); else pack=(tar); fi
names=()
builds=()
while [ $# -gt 0 ]; do
  build=$1 from=$2; shift 2
  case "$build" in
    macos-arm64|linux-x86_64)
      name="cli-$build" archive="kaiki-$build.tar.gz"
      mkdir -p "$out/$build/kaiki"
      for binary in kaiki agentic-node agentic-cli agentic-mcp; do
        install -m 755 "$from/$binary" "$out/$build/kaiki/$binary"
      done
      printf '{"build":"%s"}\n' "$name" > "$out/$build/kaiki/install.json"
      "${pack[@]}" -czf "$out/$archive" -C "$out/$build" kaiki ;;
    app-macos-arm64)
      name="$build" archive="kaiki-chat-${build#app-}.tar.gz"
      [ -d "$from/Contents/MacOS" ] || { echo "$from is not an app bundle" >&2; exit 2; }
      "${pack[@]}" -czf "$out/$archive" -C "$(dirname "$from")" "$(basename "$from")" ;;
    *) echo "unknown build $build" >&2; exit 2 ;;
  esac
  (cd "$out" && shasum -a 256 "$archive" > "$archive.sha256")
  names+=("$archive" "$archive.sha256")
  builds+=("$name" "$archive" "$(cut -d ' ' -f 1 "$out/$archive.sha256")")
  echo "packed $archive: $(cut -d ' ' -f 1 "$out/$archive.sha256")"
done
ssh_to "sudo -n docker volume inspect $volume >/dev/null" || { echo "no volume $volume: deploy the site with it first" >&2; exit 1; }
remote=$(ssh_to mktemp -d)
(cd "$out" && "${pack[@]}" -cf - "${names[@]}") | ssh_to "tar -xf - -C $remote"
ssh_to "sudo -n sh -c 'dir=\$(docker volume inspect -f {{.Mountpoint}} $volume) && mkdir -p \$dir/$version && for f in ${names[*]}; do install -m 644 $remote/\$f \$dir/$version/\$f.new && mv \$dir/$version/\$f.new \$dir/$version/\$f && install -m 644 $remote/\$f \$dir/\$f.new && mv \$dir/\$f.new \$dir/\$f; done' && rm -rf $remote"
for name in "${names[@]}"; do
  for path in "downloads/$name" "downloads/$version/$name"; do
    printf '%s ' "$SITE_URL/$path"
    curl -sS -o /dev/null -w '%{http_code}\n' "$SITE_URL/$path" || true
  done
done
# The release: this version's builds, with those published before for it.
python3 - "$version" "$SITE_URL" "${builds[@]}" <<'PY'
import json, sys
from pathlib import Path
version, site, rest = sys.argv[1], sys.argv[2].rstrip("/"), sys.argv[3:]
path = Path("deployments/release.json")
release = json.loads(path.read_text()) if path.exists() else {}
if release.get("version") != version:
    release = {"version": version, "builds": {}}
for name, archive, sha256 in zip(rest[::3], rest[1::3], rest[2::3]):
    release["builds"][name] = {"url": f"{site}/downloads/{version}/{archive}", "sha256": sha256}
release["builds"] = dict(sorted(release["builds"].items()))
path.write_text(json.dumps(release, indent=2) + "\n")
print(f"{path}: {version}, builds {', '.join(release['builds'])}")
PY
