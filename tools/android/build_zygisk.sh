#!/usr/bin/env bash
set -e

SONAME=hachimi
MODID=hachimi-edge
MODNAME=Hachimi-Edge
AUTHOR=Tenshou170
DESC="はちみーをなめると〜"
UPDATEJSON=

get_toml_value() {
    local file="$1"
    local section="$2"
    local key="$3"

    get_section() {
        local file="$1"
        local section="$2"
        sed -n "/^\[$section\]/,/^\[/p" "$file" | sed '$d'
    }
        
    get_section "$file" "$section" | grep "^$key" | sed -E 's/.*=[[:space:]]*"([^"]+)".*/\1/'
}

version_to_code() {
    local version=$1
    IFS='.' read -r major minor patch <<< "$version"

    major=$((10#$major))
    minor=$((10#$minor))
    patch=$((10#$patch))

    echo $((major * 10000 + minor * 100 + patch))
}

VERSION="$(get_toml_value Cargo.toml package version)"
GIT_COMMIT="$(git rev-parse --short HEAD)"
VERSION_STR="v$VERSION-$GIT_COMMIT"
VERSION_CODE="$(version_to_code "$VERSION")"
if [[ -z "$HACHIMI_IGNORE_DIRTY" || "$HACHIMI_IGNORE_DIRTY" != "true" ]] && [[ -n "$(git status --porcelain)" ]]
then
    VERSION_STR="$VERSION_STR-dirty"
fi

echo "*** Zygisk module: $MODNAME ($MODID)"
echo "*** Version: $VERSION_STR"
echo

echo "-- Building Android Binaries"
RELEASE=1 ./tools/android/build.sh

echo "-- Generating module"

ZYGISK_BUILD_DIR="/tmp/zygisk-build"
clean() {
    rm -rf "$ZYGISK_BUILD_DIR"
}

copy_lib() {
    local rust_lib_arch="$1"
    local mod_lib_arch="$2"
    local lib_path="build/$rust_lib_arch/$BUILD_TYPE/lib$SONAME.so"

    if [ -f "$lib_path" ]; then
        mkdir -p "$ZYGISK_BUILD_DIR/lib/$mod_lib_arch"
        cp -v "$lib_path" "$ZYGISK_BUILD_DIR/lib/$mod_lib_arch/lib$SONAME.so"

        if [[ -n "$ANDROID_NDK_ROOT" ]] && [[ -f "$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip" ]]; then
            "$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip" --strip-debug "$ZYGISK_BUILD_DIR/lib/$mod_lib_arch/lib$SONAME.so" || true
        fi
    else
        echo "Skipping optional $mod_lib_arch ($lib_path not found)"
    fi
}

if [ "$RELEASE" = "1" ]; then
    BUILD_TYPE="release"
else
    BUILD_TYPE="debug"
fi
clean

cp -r -v ./tools/android/zygisk-template "$ZYGISK_BUILD_DIR"
copy_lib aarch64-linux-android arm64-v8a

cat << EOF > "$ZYGISK_BUILD_DIR/module.prop"
id=$MODID
name=$MODNAME
version=$VERSION_STR
versionCode=$VERSION_CODE
author=$AUTHOR
description=$DESC
EOF

if [[ -n "$UPDATEJSON" ]]
then
    echo "updateJson=$UPDATEJSON" > "$ZYGISK_BUILD_DIR/module.prop"
fi

generate_sha256() {
    local file="$1"
    local hash_file="$file.sha256"
    local hash=($(sha256sum "$file"))

    echo "$hash" > "$hash_file"
    echo "$hash" "$file"
}

for f in $(find "$ZYGISK_BUILD_DIR" -type f)
do
    generate_sha256 "$f"
done

echo "-- Zipping Zygisk Module"

ZIP_FILENAME="zygisk-$MODID-$VERSION_STR-$BUILD_TYPE.zip"
ZIP_FILE="$(realpath build)/$ZIP_FILENAME"

pushd "$ZYGISK_BUILD_DIR" > /dev/null
zip -FSr6 "$ZIP_FILE" .
popd > /dev/null

echo "-- Zygisk module built successfully: build/$ZIP_FILENAME"