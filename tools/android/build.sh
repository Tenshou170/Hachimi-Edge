#!/usr/bin/env bash
set -e

case "$OSTYPE" in
    darwin*)              OS="darwin"  ;;
    linux*)               OS="linux"   ;;
    msys*|cygwin*|mingw*) OS="windows" ;;
    *)
        echo "ERROR: Unsupported host OS: $OSTYPE"
        exit 1
        ;;
esac

# Auto-detect ANDROID_NDK_ROOT / ANDROID_NDK_HOME if not explicitly set
if [[ -z "$ANDROID_NDK_ROOT" ]]; then
    if [[ -n "$ANDROID_NDK_HOME" ]]; then
        export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
    elif [[ -d "$(pwd)/ndk" ]]; then
        export ANDROID_NDK_ROOT="$(pwd)/ndk"
    fi
fi

if [[ -z "$ANDROID_NDK_ROOT" ]]; then
    echo "ERROR: ANDROID_NDK_ROOT or ANDROID_NDK_HOME must be set, or a './ndk' symlink must exist."
    exit 1
fi

TARGET_ARCH="aarch64-linux-android"

if [ "$RELEASE" = "1" ]; then
    CARGOARGS="$CARGOARGS --release"
    BUILD_TYPE="release"
else
    BUILD_TYPE="debug"
fi
export BUILD_TYPE

HOST_PREBUILT="$OS-x86_64"
TOOLCHAIN_DIR="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/$HOST_PREBUILT"
SYSROOT="$TOOLCHAIN_DIR/sysroot"

# These require the runtime-resolved NDK path so they can't live in config.toml.
# rustflags and other static flags are owned by .cargo/config.toml.
export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$SYSROOT"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TOOLCHAIN_DIR/bin/aarch64-linux-android24-clang"
export CC_aarch64_linux_android="$TOOLCHAIN_DIR/bin/aarch64-linux-android24-clang"
export CXX_aarch64_linux_android="$TOOLCHAIN_DIR/bin/aarch64-linux-android24-clang++"
export AR_aarch64_linux_android="$TOOLCHAIN_DIR/bin/llvm-ar"

mkdir -p build
cargo build --target="$TARGET_ARCH" --target-dir=build $CARGOARGS

pushd build > /dev/null

SO_NAME="libmain-arm64-v8a.so"

cp "$TARGET_ARCH/$BUILD_TYPE/libhachimi.so" "$SO_NAME"

if command -v sha256sum >/dev/null 2>&1; then
    ARM64_V8A_SHA256=($(sha256sum "$SO_NAME"))
elif command -v shasum >/dev/null 2>&1; then
    ARM64_V8A_SHA256=($(shasum -a 256 "$SO_NAME"))
else
    ARM64_V8A_SHA256=("checksum-unavailable")
fi

cat << EOF > sha256.json
{
    "$SO_NAME": "$ARM64_V8A_SHA256"
}
EOF

popd > /dev/null

echo "Android build complete: build/$SO_NAME"
