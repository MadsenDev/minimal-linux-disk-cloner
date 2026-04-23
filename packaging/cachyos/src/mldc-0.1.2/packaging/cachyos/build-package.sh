#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/../.." && pwd)"
pkgbuild="$script_dir/PKGBUILD"
pkgver="$(sed -n "s/^pkgver=//p" "$pkgbuild" | head -n1)"
tarball="$script_dir/mldc-$pkgver.tar.gz"

cd "$repo_root"

rm -f "$script_dir"/*.pkg.tar.zst "$script_dir"/*.pkg.tar.zst.sig "$tarball"
rm -rf "$script_dir/pkg" "$script_dir/src"

git ls-files --cached --others --exclude-standard -z | \
  tar \
    --exclude='packaging/cachyos/src' \
    --exclude='packaging/cachyos/src/**' \
    --exclude='packaging/cachyos/pkg' \
    --exclude='packaging/cachyos/pkg/**' \
    --exclude='packaging/cachyos/*.pkg.tar.zst' \
    --exclude='packaging/cachyos/*.pkg.tar.zst.sig' \
    --exclude='packaging/cachyos/*.tar.gz' \
    --null \
    -T - \
    -czf "$tarball" \
    --transform "s,^,mldc-$pkgver/,"

cd "$script_dir"

if pacman -Qo "$(command -v cargo)" "$(command -v rustc)" >/dev/null 2>&1; then
  makepkg -f
else
  makepkg -df
fi
