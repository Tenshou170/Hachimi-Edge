# Building Hachimi Edge

Hachimi Edge is a cross-platform game enhancement and translation mod written in Rust, supporting Windows (x64 MSVC) and Android (ARM64).

> **Supported targets only:** Linux and Windows GNU/MinGW targets are not supported. `build.rs` will hard-fail those targets immediately with a clear error message pointing to the correct commands.

---

## 1. Prerequisites

### Rust Toolchain
Install the latest stable Rust toolchain via [rustup.rs](https://rustup.rs/).

### Windows (x64 MSVC)
- **On Windows:** Standard MSVC toolchain, installed via Visual Studio or the standalone Build Tools.
- **On Linux / macOS:** `cargo-xwin`, which downloads and sets up the MSVC sysroot automatically:
  ```bash
  cargo install cargo-xwin
  ```

### Android (ARM64)
- **Android NDK r27d LTS** is recommended.
- Add the ARM64 Rust target:
  ```bash
  rustup target add aarch64-linux-android
  ```

---

## 2. NDK Setup

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

## 3. Building

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

**Output:** `build/hachimi.dll` and `build/blake3.json` (release only)

---

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

**Output:** `build/libmain-arm64-v8a.so` and `build/sha256.json` (release only)

---

## 4. Cargo Aliases Reference

Defined in `.cargo/config.toml`. For day-to-day development only — use the build scripts above for producing release artifacts.

| Alias | Description |
|---|---|
| `cargo xcheck` | Compiler check for Windows MSVC target (release profile) |
| `cargo xbuild` | Build for Windows MSVC (no `--release`; use the script for releases) |
| `cargo xclippy` | Clippy lint for Windows MSVC target |
| `cargo acheck` | Compiler check for Android ARM64 target (release profile) |
| `cargo abuild` | Build for Android ARM64 (no `--release`; use the script for releases) |
| `cargo aclippy` | Clippy lint for Android ARM64 target |
