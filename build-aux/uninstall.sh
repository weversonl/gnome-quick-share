#!/bin/sh
# Uninstalls the application using Ninja or Meson and updates system caches.

set -e

# Prefix for paths (can be overridden)
PREFIX="${MESON_INSTALL_PREFIX:-/usr/local}"

# Check if we have a build directory
BUILD_DIR="${1:-build}"

if [ ! -d "$BUILD_DIR" ]; then
    echo "Error: Build directory '$BUILD_DIR' not found. Please provide it as the first argument."
    echo "Usage: $0 <build_dir> [prefix]"
    exit 1
fi

if [ -n "$2" ]; then
    PREFIX="$2"
fi

# Run ninja uninstall if ninja is available, otherwise use meson compile
echo "Removing installed files..."
if command -v ninja >/dev/null 2>&1; then
    sudo ninja -C "$BUILD_DIR" uninstall
else
    sudo meson compile -C "$BUILD_DIR" uninstall
fi

# Update system caches
echo "Updating system caches in $PREFIX/share..."

if command -v glib-compile-schemas >/dev/null 2>&1; then
    sudo glib-compile-schemas "$PREFIX/share/glib-2.0/schemas" || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    sudo gtk-update-icon-cache -q -f "$PREFIX/share/icons/hicolor" || true
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    sudo update-desktop-database -q "$PREFIX/share/applications" || true
fi

echo "Uninstallation complete."
