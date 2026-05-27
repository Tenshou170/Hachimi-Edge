# Building Hachimi Edge

Hachimi Edge is a cross-platform game enhancement and translation mod written in Rust, supporting Windows (x64) and Android (ARM64).

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

### Android Support
- **Android NDK**: **r27d LTS** (Long-Term Support) is highly recommended.
- **Rust Target**: Add the ARM64 Android cross-compilation target:
  ```bash
  rustup target add aarch64-linux-android
  ```

---

## 2. Dependency Setup (Required for Visual Parity)

To achieve absolute visual parity with the official release builds (such as the custom Combo Box sizing and UI rendering), Hachimi Edge compiles against custom-patched versions of Egui.

This is **fully automated** via Cargo. `Cargo.toml` is pre-configured to automatically fetch, cache, and apply these patches from the git fork repository (`THShafi170/egui` on branch `hachimi-patches`) upon compilation. No manual cloning, patching, or local setup script is required!

---

## 3. Local NDK Environment Setup

To keep the repository clean and avoid hardcoded absolute paths, Hachimi's build configuration uses a symbolic link named `ndk` in the project root pointing to your Android NDK directory.

### On Linux / macOS:
Create the symlink pointing to your extracted NDK folder (for example, `r27d`):
```bash
ln -s /home/user/ndk/android-ndk-r27d ndk
```

### On Windows:
Create a directory junction using Command Prompt or PowerShell:
```cmd
mklink /J ndk C:\path\to\android-ndk-r27d
```

*Note: The `ndk` link is automatically ignored by Git.*

---

## 4. Compiling the Mod

### Windows (x64)

#### Building on Windows:
```bash
cargo build --target x86_64-pc-windows-msvc --release
```

#### Cross-compiling on Linux:
Use our pre-configured Cargo alias (which executes `cargo-xwin`):
```bash
cargo xbuild
```

**Output**: `target/x86_64-pc-windows-msvc/release/hachimi.dll`

---

### Android (ARM64)

#### Building locally:
Run our pre-configured Cargo alias (requires the NDK symlink setup in Step 3):
```bash
cargo abuild
```
*Note: This builds using unified **API Level 24** and targets **16KB page size alignment**, guaranteeing complete backward compatibility down to Android 7.0 and forward compatibility with Android 15.*

#### Building using CI-script locally (requires setting `ANDROID_NDK_ROOT`):
If you wish to run the identical script utilized by our GitHub Actions runner:
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
RELEASE=1 ./tools/android/build.sh
```

**Output**: `target/aarch64-linux-android/release/libhachimi.so`

---

## Summary of Useful Cargo Aliases

These aliases are defined in `.cargo/config.toml` for standardizing developer workflows:

*   `cargo abuild`: Compiles Android in release mode using the local `ndk` symlink.
*   `cargo acheck`: Quick compiler-check for the Android target.
*   `cargo xbuild`: Runs `cargo xwin` to cross-compile the Windows version on a Linux host.
*   `cargo xcheck`: Quick compiler-check for the Windows cross-compilation target.
