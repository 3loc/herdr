#!/bin/sh
set -eu

repo="3loc/herdr"
install_dir="${HERDR_INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s)" in
    Linux) platform="linux" ;;
    Darwin) platform="macos" ;;
    *)
        printf '%s\n' "error: this installer supports Linux and macOS" >&2
        exit 1
        ;;
esac

case "$(uname -m)" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *)
        printf 'error: unsupported architecture: %s\n' "$(uname -m)" >&2
        exit 1
        ;;
esac

asset="herdr-${platform}-${arch}"
url="https://github.com/${repo}/releases/latest/download/${asset}"
checksum_url="${url}.sha256"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM

if command -v curl >/dev/null 2>&1; then
    curl -fL --retry 3 --proto '=https' --tlsv1.2 -o "$tmp_dir/herdr.sha256" "$checksum_url"
    curl -fL --retry 3 --proto '=https' --tlsv1.2 -o "$tmp_dir/herdr" "$url"
elif command -v wget >/dev/null 2>&1; then
    wget -O "$tmp_dir/herdr.sha256" "$checksum_url"
    wget -O "$tmp_dir/herdr" "$url"
else
    printf '%s\n' "error: curl or wget is required" >&2
    exit 1
fi

expected="$(awk 'NR == 1 { print $1 }' "$tmp_dir/herdr.sha256")"
case "$expected" in
    *[!0-9a-fA-F]*|'')
        printf '%s\n' "error: release checksum is invalid" >&2
        exit 1
        ;;
esac
if [ "${#expected}" -ne 64 ]; then
    printf '%s\n' "error: release checksum is invalid" >&2
    exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$tmp_dir/herdr" | awk '{ print $1 }')"
elif command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$tmp_dir/herdr" | awk '{ print $1 }')"
else
    printf '%s\n' "error: sha256sum or shasum is required" >&2
    exit 1
fi

if [ "$actual" != "$expected" ]; then
    printf '%s\n' "error: release checksum verification failed" >&2
    exit 1
fi

mkdir -p "$install_dir"
chmod 755 "$tmp_dir/herdr"
mv "$tmp_dir/herdr" "$install_dir/herdr"

printf 'installed 3LOC Herdr to %s/herdr\n' "$install_dir"
case ":${PATH}:" in
    *":${install_dir}:"*) ;;
    *) printf 'add %s to PATH, then run: herdr\n' "$install_dir" ;;
esac
