#!/bin/bash
# Packs the command line of one platform for kaikichat.com/downloads, as
# install.sh unpacks it and kaiki updates itself from it:
#   pack-cli.sh macos-arm64|linux-x86_64 FROM OUT
# makes OUT/kaiki-PLATFORM.tar.gz: a kaiki/ directory with kaiki,
# kaiki-agentic-node, agentic-cli and agentic-mcp from FROM, and install.json,
# the marker that lets that install update itself. agentic-node, the daemon's
# former name, stays as a link to it: kaiki 0.2.4 and older refuse an update
# without that file. OUT/PLATFORM is the staging directory; it must not exist.
set -euo pipefail
[ $# -eq 3 ] || { echo "usage: $0 macos-arm64|linux-x86_64 FROM OUT" >&2; exit 2; }
platform=$1 from=$2 out=$3
case "$platform" in
  macos-arm64|linux-x86_64) ;;
  *) echo "unknown platform $platform" >&2; exit 2 ;;
esac
export COPYFILE_DISABLE=1
# No macOS extended attributes in the archives: Linux tar warns about them.
if tar --version 2>/dev/null | grep -q bsdtar; then pack=(tar --no-xattrs --no-mac-metadata); else pack=(tar); fi
mkdir -p "$out"
stage="$out/$platform"
mkdir "$stage" "$stage/kaiki"
for binary in kaiki kaiki-agentic-node agentic-cli agentic-mcp; do
  install -m 755 "$from/$binary" "$stage/kaiki/$binary"
done
ln -s kaiki-agentic-node "$stage/kaiki/agentic-node"
printf '{"build":"cli-%s"}\n' "$platform" > "$stage/kaiki/install.json"
"${pack[@]}" -czf "$out/kaiki-$platform.tar.gz" -C "$stage" kaiki
