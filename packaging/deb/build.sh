#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/app/gtk/Cargo.toml" | head -n1)"
arch="$(dpkg --print-architecture)"
pkgroot="$root/packaging/out/deb/gnome-quick-share_${version}_${arch}"
artifact="$root/packaging/out/deb/gnome-quick-share_${version}_${arch}.deb"
build_dir="$root/packaging/out/deb/meson-build"

rm -rf "$pkgroot" "$artifact" "$build_dir"
mkdir -p "$pkgroot/DEBIAN"

cd "$root"
meson setup \
  --prefix=/usr \
  --buildtype=release \
  "$build_dir"

ninja -C "$build_dir"

DESTDIR="$pkgroot" meson install -C "$build_dir"

cat > "$pkgroot/DEBIAN/control" <<EOF
Package: gnome-quick-share
Version: $version
Section: utils
Priority: optional
Architecture: $arch
Maintainer: weversonl
Depends: libgtk-4-1, libadwaita-1-0, libgtk-3-0, libayatana-appindicator3-1, libdbus-1-3, libglib2.0-bin
Description: GNOME Quick Share client
 GTK4 and Libadwaita desktop client for nearby file sharing.
EOF

install -Dm755 "$root/packaging/deb/postinst" "$pkgroot/DEBIAN/postinst"
install -Dm755 "$root/packaging/deb/postrm"   "$pkgroot/DEBIAN/postrm"

dpkg-deb --build --root-owner-group "$pkgroot" "$artifact"
echo "Built $artifact"
