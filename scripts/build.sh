#!/bin/bash
# HyprDesk build script — compiles in release mode and copies the binary to build/.
# HyprDesk script de compilación — compila en modo release y copia el binario a build/.

set -e
cd "$(dirname "$0")/.."
cargo build --release
mkdir -p build
cp .buildcache/release/hyprdesk build/hyprdesk
echo "Built: build/hyprdesk"
