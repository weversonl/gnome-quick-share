#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
manifest="$root/packaging/flatpak/io.github.weversonl.GnomeQS.json"
vendor_dir="$root/packaging/flatpak/vendor"
build_dir="$root/packaging/out/flatpak/build"
repo_dir="$root/packaging/out/flatpak/repo"
bundle="$root/packaging/out/flatpak/io.github.weversonl.GnomeQS.flatpak"

rm -rf "$vendor_dir" "$build_dir" "$repo_dir" "$bundle"
mkdir -p "$vendor_dir"

cargo vendor --locked "$vendor_dir" > /dev/null

flatpak-builder \
  --force-clean \
  --repo="$repo_dir" \
  "$build_dir" \
  "$manifest"

flatpak build-bundle "$repo_dir" "$bundle" io.github.weversonl.GnomeQS
echo "Built $bundle"
