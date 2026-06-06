#!/usr/bin/env bash
# Build the Native32Emu libretro core and stage it for RetroArch.
#
# RetroArch requires the core file name to end in `_libretro` and the matching
# info file to share the same base name. Cargo names the cdylib after the lib
# target (`native32emu`), so this script renames the artifact to
# `native32emu_libretro.<ext>` and copies the info file alongside it into dist/.
#
# Usage:
#   ./scripts/build-libretro.sh            # release build into ./dist
#   ./scripts/build-libretro.sh --debug    # debug build

set -euo pipefail

PROFILE="release"
OUT_DIR="dist"
CORE_NAME="native32emu_libretro"

for arg in "$@"; do
    case "$arg" in
        --debug) PROFILE="debug" ;;
        --out=*) OUT_DIR="${arg#*=}" ;;
        *) echo "Unknown argument: $arg" >&2; exit 1 ;;
    esac
done

# Resolve repo root (parent of this script's directory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$REPO_ROOT"

echo "Building libretro core ($PROFILE)..."
build_args=(rustc --lib --features libretro --no-default-features --crate-type cdylib)
if [ "$PROFILE" = "release" ]; then
    build_args+=(--release)
fi
cargo "${build_args[@]}"

# Locate the produced dynamic library across platforms
artifact_dir="target/$PROFILE"
src=""
for c in "native32emu.dll" "libnative32emu.so" "libnative32emu.dylib"; do
    if [ -f "$artifact_dir/$c" ]; then
        src="$artifact_dir/$c"
        break
    fi
done
if [ -z "$src" ]; then
    echo "Could not find built dynamic library in $artifact_dir" >&2
    exit 1
fi

# Target file name keeps the platform extension but uses the _libretro base name
ext="${src##*.}"
dest_name="$CORE_NAME.$ext"

mkdir -p "$OUT_DIR"
cp -f "$src" "$OUT_DIR/$dest_name"
cp -f "native32emu_libretro.info" "$OUT_DIR/native32emu_libretro.info"

echo "Staged libretro core into $OUT_DIR/:"
echo "  - $dest_name"
echo "  - native32emu_libretro.info"
echo ""
echo "Install: copy '$dest_name' to RetroArch's cores/ directory and"
echo "'native32emu_libretro.info' to RetroArch's info/ directory."
