#!/usr/bin/env bash
set -e

case "$OSTYPE" in
    msys*|cygwin*|mingw*)
        echo "Building natively for Windows MSVC..."
        cargo build --target x86_64-pc-windows-msvc --target-dir build --release
        ;;
    *)
        echo "Cross-building for Windows MSVC via cargo-xwin..."
        if ! command -v cargo-xwin &> /dev/null; then
            echo "ERROR: cargo-xwin is required for cross-compiling to Windows MSVC on non-Windows hosts."
            echo "Install it using: cargo install cargo-xwin"
            exit 1
        fi
        cargo xwin build --target x86_64-pc-windows-msvc --target-dir build --release
        ;;
esac

mkdir -p build
cp build/x86_64-pc-windows-msvc/release/hachimi.dll build/hachimi.dll

if command -v b3sum >/dev/null 2>&1; then
    DLL_HASH=$(b3sum build/hachimi.dll | awk '{print $1}')
    cat << EOF > build/blake3.json
{
    "hachimi.dll": "$DLL_HASH"
}
EOF
fi

echo "Windows build complete: build/hachimi.dll"
