#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
manifest="$root/packaging/flatpak/io.github.weversonl.GnomeQuickShare.json"
build_dir="$root/packaging/out/flatpak/build"
repo_dir="$root/packaging/out/flatpak/repo"
bundle="$root/packaging/out/flatpak/io.github.weversonl.GnomeQuickShare.flatpak"

rm -rf "$build_dir" "$repo_dir" "$bundle"

flatpak-builder \
  --force-clean \
  --repo="$repo_dir" \
  "$build_dir" \
  "$manifest"

flatpak build-bundle "$repo_dir" "$bundle" io.github.weversonl.GnomeQuickShare
echo "Built $bundle"
