#!/bin/sh
# Installs Kaiki Chat's command line for this user, without sudo:
#   curl -fsSL https://kaikichat.com/install.sh | sh
# It puts `kaiki` and the node it runs into ~/.local/share/kaiki/bin, checks
# the download against its published SHA-256, and adds a `kaiki` launcher to
# ~/.local/bin. Run it again to update. macOS on Apple silicon and Linux
# x86_64 (glibc 2.39 or later, such as Ubuntu 24.04).
set -eu

base="${KAIKI_DOWNLOADS:-https://kaikichat.com/downloads}"
home="${KAIKI_HOME:-$HOME/.local/share/kaiki}"
bindir="${KAIKI_BIN:-$HOME/.local/bin}"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) platform=macos-arm64 ;;
  Linux-x86_64) platform=linux-x86_64 ;;
  *)
    echo "kaiki: no build for $(uname -s) $(uname -m) yet; there are macOS on Apple silicon and Linux x86_64." >&2
    exit 1 ;;
esac

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
archive="kaiki-$platform.tar.gz"
curl -fsSL "$base/$archive" -o "$tmp/$archive"
curl -fsSL "$base/$archive.sha256" -o "$tmp/$archive.sha256"

expected=$(cut -d ' ' -f 1 "$tmp/$archive.sha256")
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$tmp/$archive" | cut -d ' ' -f 1)
else
  actual=$(shasum -a 256 "$tmp/$archive" | cut -d ' ' -f 1)
fi
if [ "$expected" != "$actual" ]; then
  echo "kaiki: the download does not match its published SHA-256; nothing was installed." >&2
  exit 1
fi

tar -xzf "$tmp/$archive" -C "$tmp"
mkdir -p "$home" "$bindir"
rm -rf "$home/bin.new"
mv "$tmp/kaiki" "$home/bin.new"
rm -rf "$home/bin"
mv "$home/bin.new" "$home/bin"

# A launcher, not a link: kaiki finds its node beside its own path.
cat > "$bindir/kaiki" <<LAUNCHER
#!/bin/sh
exec "$home/bin/kaiki" "\$@"
LAUNCHER
chmod 755 "$bindir/kaiki"

version=$("$home/bin/kaiki" --version 2>/dev/null | awk '{print $NF}')
echo "Installed kaiki ${version:-}: $home/bin/kaiki"
case ":$PATH:" in
  *":$bindir:"*) echo "Run: kaiki skill show" ;;
  *) echo "Run: $home/bin/kaiki skill show   ($bindir is not on PATH)" ;;
esac
