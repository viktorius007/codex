#!/usr/bin/env bash
set -euo pipefail
base=/scratch/agent/stable170
mkdir -p "$base/debs" "$base/sysroot" "$base/tmp"
cd "$base/debs"
# Pin plugins-base to the available release pocket; the stale updates index
# advertises 1.28.2-1ubuntu0.2 files that return HTTP 404.
apt-get download libgstreamer1.0-dev libgstreamer1.0-0 \
  libgstreamer-gl1.0-0=1.28.2-1 libgstreamer-plugins-base1.0-0=1.28.2-1 \
  libgstreamer-plugins-base1.0-dev=1.28.2-1 libglib2.0-dev \
  libglib2.0-dev-bin libglib2.0-0t64 libgio-2.0-dev liborc-0.4-dev \
  liborc-0.4-0t64 libffi-dev libpcre2-dev libmount-dev libblkid-dev \
  libsysprof-capture-4-dev
for package in *.deb; do dpkg-deb -x "$package" "$base/sysroot"; done
python3 - <<'PY'
from pathlib import Path
root = Path('/scratch/agent/stable170/sysroot')
for path in root.rglob('*.pc'):
    path.write_text(path.read_text().replace('prefix=/usr', f'prefix={root}/usr'))
PY
PKG_CONFIG_PATH="$base/sysroot/usr/lib/x86_64-linux-gnu/pkgconfig" \
  pkg-config --modversion gstreamer-1.0 gstreamer-app-1.0 gstreamer-audio-1.0
