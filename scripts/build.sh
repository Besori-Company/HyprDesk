#!/bin/bash
# HyprDesk build script — compiles in release mode and copies the binary to build/.
# HyprDesk script de compilación — compila en modo release y copia el binario a build/.

set -e
cd "$(dirname "$0")/.."
cargo build --release

# .cargo/config.toml is gitignored: a fresh clone builds into target/ / .cargo/config.toml está en el .gitignore: un clon nuevo compila en target/
BUILT=""
for DIR in .buildcache/release target/release; do
    if [ -f "$DIR/hyprdesk" ]; then BUILT="$DIR/hyprdesk"; break; fi
done

if [ -z "$BUILT" ]; then
    echo "Error: compiled binary not found / Error: no se encontró el binario compilado" >&2
    exit 1
fi

mkdir -p build
cp "$BUILT" build/hyprdesk
echo "Built: build/hyprdesk"
