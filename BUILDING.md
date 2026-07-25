# Building Hachimi Edge

Hachimi Edge is a cross-platform game enhancement and translation mod written in Rust, supporting Windows (x64) and Android (ARM64).

> **Supported targets only:** Hachimi Edge does not support Linux builds or Windows GNU/MinGW builds. The build script intentionally hard-fails those targets so accidental `cargo check --target x86_64-unknown-linux-gnu` or `cargo check --target x86_64-pc-windows-gnu` runs stop immediately with the supported commands.

---

## 1. Prerequisites

### Rust Toolchain
Install the latest stable Rust toolchain via [rustup.rs](https://rustup.rs/).

### Windows Support
- **Native (on Windows)**: Standard MSVC toolchain (installed via Visual Studio or Build Tools).
- **Cross-compilation (on Linux)**: `cargo-xwin` is required to build for Windows on Linux hosts:
  ```bash
  cargo install cargo-xwin
  ```
- **Unsupported**: Windows GNU/MinGW targets are blocked. Use `x86_64-pc-windows-msvc` through the commands below.

### Android Support
- **Android NDK**: **r27d LTS** (Long-Term Support) is highly recommended.
- **Rust Target**: Add the ARM64 Android cross-compilation target:
  ```bash
  rustup target add aarch64-linux-android
  ```

---

## 2. Dependency Setup (Required for Visual Parity)

To achieve absolute visual parity with the official release builds (such as the custom Combo Box sizing and UI rendering), Hachimi Edge compiles against custom-patched versions of Egui.

This is **fully automated** via Cargo. `Cargo.toml` is pre-configured to automatically fetch, cache, and apply these patches from the git fork repository (`Tenshou170/egui` on `main` branch) upon compilation. No manual cloning, patching, or local setup script is required!

---

## 3. NDK Environment Setup (Zero-Config)

Hachimi's build configuration automatically discovers your Android NDK path in order of precedence:
1. `$ANDROID_NDK_ROOT` environment variable
2. `$ANDROID_NDK_HOME` environment variable
3. A local `./ndk` symlink or directory in the project root

### Quick Setup Option A (Environment Variable - Recommended):
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
```

### Quick Setup Option B (Symlink / Junction):
- **Linux / macOS**: `ln -s /path/to/android-ndk-r27d ndk`
- **Windows**: `mklink /J ndk C:\path\to\android-ndk-r27d`

---

## 4. Compiling the Mod

### Windows (x64 MSVC)

#### Quick check:
```bash
cargo xcheck
```

#### Build DLL & release package:
```bash
./tools/windows/build.sh
```
*(On Windows hosts, this builds natively. On Linux/macOS hosts, this cross-compiles via `cargo-xwin`).*

**Output**: `build/hachimi.dll` and `build/blake3.json`

---

### Android (ARM64)

#### Quick check:
```bash
cargo acheck
```

#### Build SO & release package:
```bash
RELEASE=1 ./tools/android/build.sh
```
*Note: This builds using unified **API Level 24** and targets **16KB page size alignment**, guaranteeing complete backward compatibility down to Android 7.0 and forward compatibility with Android 15.*

**Output**: `build/libmain-arm64-v8a.so` and `build/sha256.json`

---

## Summary of Cargo & Script Aliases

These aliases and scripts are defined to standardize developer workflows across platforms:

*   `cargo acheck`: Quick compiler check for the Android target.
*   `cargo aclippy`: Clippy lint check for the Android target.
*   `cargo xcheck`: Quick compiler check for Windows target (runs natively on Windows; `cargo-xwin` on Linux/macOS).
*   `cargo xclippy`: Clippy lint check for Windows target.
*   `./tools/android/build.sh`: Builds Android ARM64 `.so` binary and SHA256 checksums.
*   `./tools/windows/build.sh`: Builds Windows MSVC `.dll` binary and BLAKE3 checksums.

Raw Linux and Windows GNU target commands are intentionally unsupported and will fail in `build.rs`. Use the aliases and scripts above for routine checks and builds.
