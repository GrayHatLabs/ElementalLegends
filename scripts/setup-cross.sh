#!/usr/bin/env bash
# One-time setup (Ubuntu 22.04 / WSL) for cross-compiling to aarch64 handhelds
# such as the Anbernic RG35XX H. Needs root (or sudo).
set -e
SUDO=""; [ "$(id -u)" -ne 0 ] && SUDO="sudo"
CODENAME=$(. /etc/os-release && echo "$VERSION_CODENAME")

# Existing amd64 mirrors don't carry arm64 packages: pin them to amd64 (backup kept)
# and add Ubuntu Ports for arm64.
if ! grep -q "arch=amd64" /etc/apt/sources.list; then
  $SUDO cp /etc/apt/sources.list /etc/apt/sources.list.bak-elementallegends
  $SUDO sed -i -E 's/^deb (http|https|mirror)/deb [arch=amd64] \1/' /etc/apt/sources.list
fi
$SUDO tee /etc/apt/sources.list.d/arm64-ports.list >/dev/null <<EOF
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports ${CODENAME} main restricted universe multiverse
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports ${CODENAME}-updates main restricted universe multiverse
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports ${CODENAME}-security main restricted universe multiverse
EOF
$SUDO dpkg --add-architecture arm64
$SUDO apt-get update || true
$SUDO env DEBIAN_FRONTEND=noninteractive apt-get install -y gcc-aarch64-linux-gnu libc6-dev-arm64-cross libsdl2-dev:arm64

export PATH="$HOME/.cargo/bin:$PATH"
rustup target add aarch64-unknown-linux-gnu
echo "Cross toolchain ready."
