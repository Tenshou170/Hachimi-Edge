#!/usr/bin/env bash
set -e

if [ "$RELEASE" = "1" ]; then
    CARGOARGS="$CARGOARGS --release"
    BUILD_TYPE="release"
else
    BUILD_TYPE="debug"
fi

case "$OSTYPE" in
    msys*|cygwin*|mingw*)
        # Native Windows host — MSVC toolchain is available directly.
        echo "Building natively for Windows MSVC..."
        cargo build --target-dir build $CARGOARGS
        ;;
    darwin*|linux*)
        # Non-Windows host — cross-compile via cargo-xwin which sets up the
        # MSVC sysroot and CL_FLAGS/LIB environment that build.rs requires.
        echo "Cross-building for Windows MSVC via cargo-xwin..."
        if ! command -v cargo-xwin &> /dev/null; then
            echo "ERROR: cargo-xwin is required for cross-compiling to Windows MSVC."
            echo "Install it with: cargo install cargo-xwin"
            exit 1
        fi
        cargo xbuild --target-dir build $CARGOARGS
        ;;
    *)
        echo "ERROR: Unsupported host OS: $OSTYPE"
        exit 1
        ;;
esac

mkdir -p build
cp build/x86_64-pc-windows-msvc/$BUILD_TYPE/hachimi.dll build/hachimi.dll

if command -v b3sum >/dev/null 2>&1; then
    DLL_HASH=$(b3sum build/hachimi.dll | awk '{print $1}')
    cat << EOF > build/blake3.json
{
    "hachimi.dll": "$DLL_HASH"
}
EOF
fi

echo "Windows build complete: build/hachimi.dll"
