#!/usr/bin/env bash
# Cross-compile for aarch64 Linux handhelds (Anbernic RG35XX H etc.) and assemble
# a PortMaster-style port folder in dist/.
set -e
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR="$HOME/.cache/elementallegends-target"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
export RUSTFLAGS="-L /usr/lib/aarch64-linux-gnu"
cargo build --release --target aarch64-unknown-linux-gnu

OUT=dist/ports
rm -rf "$OUT"
mkdir -p "$OUT/elementallegends"
cp "$CARGO_TARGET_DIR/aarch64-unknown-linux-gnu/release/elementallegends" "$OUT/elementallegends/"
cp port/ElementalLegends.sh "$OUT/"
cp port/elementallegends.gptk "$OUT/elementallegends/"
# Menu art for the Ports list (cover + screenshot, PortMaster convention).
cp port/cover.png port/screenshot.png "$OUT/elementallegends/"
cp README.md "$OUT/elementallegends/"
chmod +x "$OUT/ElementalLegends.sh" "$OUT/elementallegends/elementallegends"
(cd dist && rm -f ElementalLegends-aarch64.zip && python3 -m zipfile -c ElementalLegends-aarch64.zip ports/)
file "$OUT/elementallegends/elementallegends" || true
echo "Built dist/ElementalLegends-aarch64.zip"
