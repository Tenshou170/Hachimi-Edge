# Building Hachimi Edge

Hachimi Edge is a cross-platform game enhancement and translation mod written in Rust, supporting Windows (x64 MSVC) and Android (ARM64). This guide covers the toolchain, environment setup, and every command needed to produce working artifacts.

> **Supported targets only:** Linux and Windows GNU/MinGW targets are not supported. `build.rs` will hard-fail those targets immediately with a clear error message pointing to the correct commands.

> [!NOTE]
> The full release pipeline — including the Windows installer and Cellar builds — lives in [`.github/workflows/test_build.yml`](.github/workflows/test_build.yml), which is the authoritative reference for how official artifacts are produced.

---

## 1. Getting the Source

```bash
git clone https://github.com/Tenshou170/Hachimi-Edge.git
cd Hachimi-Edge
```

If the git working tree has uncommitted changes, the Zygisk build appends a `-dirty` suffix to the module version. Set `HACHIMI_IGNORE_DIRTY=true` to suppress this (CI pins it to `false` so local modifications stay visible in artifacts).

---

## 2. Prerequisites

### Rust Toolchain
Install the latest stable Rust toolchain via [rustup.rs](https://rustup.rs/).

### Windows (x64 MSVC)
- **On Windows:** Standard MSVC toolchain, installed via Visual Studio or the standalone Build Tools.
- **On Linux / macOS:** `cargo-xwin`, which downloads and sets up the MSVC sysroot automatically:
  ```bash
  cargo install cargo-xwin
  ```
  The LLVM/Clang/LLD toolchain is also required for the link stage; make sure `llvm-lib` is available (CI symlinks it to `llvm-ar`):
  ```bash
  # Debian / Ubuntu (LLVM 19, as used in CI)
  sudo apt-get install -y clang-19 lld-19 llvm-19
  sudo ln -sf /usr/lib/llvm-19/bin/llvm-ar /usr/local/bin/llvm-lib

  # macOS
  brew install llvm
  sudo ln -sf "$(brew --prefix llvm)/bin/llvm-ar" /usr/local/bin/llvm-lib
  ```

### Android (ARM64)
- **Android NDK r27d LTS** is recommended.
- Add the ARM64 Rust target:
  ```bash
  rustup target add aarch64-linux-android
  ```
- `zip` is required on the host when packaging the Zygisk module.

---

## 3. NDK Setup

The build system discovers the NDK path automatically, in order of precedence:

1. `$ANDROID_NDK_ROOT` environment variable
2. `$ANDROID_NDK_HOME` environment variable
3. A `./ndk` symlink or directory at the project root

**Option A — Environment variable (recommended for CI and one-off builds):**
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
```

**Option B — Local symlink (recommended for day-to-day development):**
```bash
# Linux / macOS
ln -s /path/to/android-ndk-r27d ndk

# Windows (Command Prompt)
mklink /J ndk C:\path\to\android-ndk-r27d
```

No manual edits to `.cargo/config.toml` are needed — the build system handles all host platforms automatically.

---

## 4. Building

### Windows (x64 MSVC)

**Compiler check:**
```bash
cargo xcheck
```

**Debug build:**
```bash
./tools/windows/build.sh
```

**Release build:**
```bash
RELEASE=1 ./tools/windows/build.sh
```

The script detects the host OS automatically:
- On Windows: builds natively using the MSVC toolchain.
- On Linux / macOS: cross-compiles via `cargo-xwin`.

**Output:** `build/hachimi.dll`, plus `build/blake3.json` (generated when `b3sum` is installed).

### Android (ARM64)

**Compiler check:**
```bash
cargo acheck
```

**Debug build:**
```bash
./tools/android/build.sh
```

**Release build:**
```bash
RELEASE=1 ./tools/android/build.sh
```

Builds against **API level 24** with **16 KB page-size alignment**, giving full compatibility from Android 7.0 through Android 15+.

**Output:** `build/libmain-arm64-v8a.so`, plus `build/sha256.json` (generated when `sha256sum` or `shasum` is available).

### Zygisk Module (Android)

```bash
RELEASE=1 ./tools/android/build_zygisk.sh
```

Runs the Android release build, strips debug symbols with the NDK's `llvm-strip`, packages the module from `tools/android/zygisk-template`, and writes per-file SHA-256 checksums.

**Output:** `build/zygisk-hachimi-edge-v<version>-<commit>[-dirty]-release.zip` (a Magisk/KernelSU-installable module). Requires `zip` on the host.

### Extra Cargo Arguments

Both build scripts forward additional arguments to cargo via `CARGOARGS`:

```bash
RELEASE=1 CARGOARGS="--features some_feature" ./tools/windows/build.sh
```

---

## 5. Cargo Aliases Reference

Defined in `.cargo/config.toml`. The `check`, `clippy`, and `test` aliases mirror the commands in CI byte-for-byte, including `-D warnings`, so a local run reproduces CI exactly. For producing release artifacts, use the build scripts above instead of the build aliases.

| Alias | Description |
|---|---|
| `cargo xcheck` | Compiler check for Windows MSVC target (release profile) |
| `cargo xbuild` | Build for Windows MSVC via `cargo-xwin` (release profile) |
| `cargo xclippy` | Clippy lint for Windows MSVC target (`-D warnings`) |
| `cargo xtest` | Compile tests for Windows MSVC target (`--no-run`) |
| `cargo acheck` | Compiler check for Android ARM64 target (release profile) |
| `cargo abuild` | Build for Android ARM64 (release profile) |
| `cargo aclippy` | Clippy lint for Android ARM64 target (`-D warnings`) |
| `cargo atest` | Compile tests for Android ARM64 target (`--no-run`) |
