#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/app/gtk/Cargo.toml" | head -n1)"
topdir="$root/packaging/out/rpm/rpmbuild"
source="$topdir/SOURCES/gnome-quick-share-${version}.tar.gz"

rm -rf "$topdir"
mkdir -p "$topdir"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

tar \
  --exclude='./target' \
  --exclude='./packaging/out' \
  --exclude-vcs \
  --transform "s,^\.,gnome-quick-share-${version}," \
  -czf "$source" \
  -C "$root" .

install -Dm644 "$root/packaging/rpm/gnome-quick-share.spec" "$topdir/SPECS/gnome-quick-share.spec"

rpmbuild -ba "$topdir/SPECS/gnome-quick-share.spec" \
  --define "_topdir $topdir" \
  --define "version_override $version"

echo "Built RPMs in $topdir/RPMS"
