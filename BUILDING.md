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

> [!IMPORTANT]
> **Host OS Configuration (`.cargo/config.toml`)**:
> Depending on whether your host machine is Windows or Linux/macOS, you will need to toggle the active configuration lines inside [.cargo/config.toml](file:///.cargo/config.toml).
> - **Windows Developers**: Keep the default Windows lines active (active by default).
> - **Linux/macOS Developers**: Comment out the Windows lines and uncomment the commented Linux blocks under `[alias]`, `[target.aarch64-linux-android]`, and `[env]`.

---

## 4. Compiling the Mod

### Windows (x64)

#### Checking the code:
Use our pre-configured Cargo alias:
```bash
cargo xcheck
```
*(On Windows hosts, this checks natively. On Linux/macOS hosts, this cross-checks via `cargo-xwin`).*

#### Building the DLL:
Use our pre-configured Cargo alias:
```bash
cargo xbuild
```
*(On Windows hosts, this builds natively. On Linux/macOS hosts, this cross-compiles via `cargo-xwin`).*

**Output**: `target/x86_64-pc-windows-msvc/release/hachimi.dll`

---

### Android (ARM64)

#### Checking locally:
Run the Android check alias:
```bash
cargo acheck
```

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
*   `cargo xbuild`: Builds the Windows version (runs natively on Windows hosts; runs `cargo-xwin` to cross-compile on Linux/macOS hosts).
*   `cargo xcheck`: Quick compiler-check for the Windows target (runs natively on Windows hosts; runs `cargo-xwin` to cross-check on Linux/macOS hosts).

Raw Linux and Windows GNU target commands are intentionally unsupported and will fail in `build.rs`. Use the aliases above for routine checks and builds.
