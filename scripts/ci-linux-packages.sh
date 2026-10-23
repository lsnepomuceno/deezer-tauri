#!/bin/sh
# Installs the system packages the Linux runners need (CI and release).
# The runners' Azure mirror stalled for half an hour on 2026-10-23, so each attempt
# is bounded, and the second one goes to archive.ubuntu.com instead.
set -eu

packages="libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev patchelf
gstreamer1.0-plugins-good gstreamer1.0-libav $*"

attempt() {
  sudo timeout 300 apt-get -o Acquire::Retries=3 -o Acquire::http::Timeout=30 update &&
    sudo timeout 600 apt-get -o Acquire::Retries=3 -o Acquire::http::Timeout=30 \
      install -y --no-install-recommends $packages
}

if attempt; then
  exit 0
fi

echo "The default mirror failed; retrying on archive.ubuntu.com" >&2
for list in /etc/apt/sources.list /etc/apt/sources.list.d/*.list /etc/apt/sources.list.d/*.sources; do
  [ -f "$list" ] && sudo sed -i 's|http://azure.archive.ubuntu.com|http://archive.ubuntu.com|g' "$list"
done
attempt
