#!/bin/bash
set -e
cd "$(dirname "$0")/.."
cargo build --release
mkdir -p build
cp .buildcache/release/hyprdesk build/hyprdesk
echo "Built: build/hyprdesk"
