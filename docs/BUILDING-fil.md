# Gabay sa Pagbu-build ng Hachimi Edge

Ang Hachimi Edge ay isang cross-platform na game enhancement at translation mod na nakasulat sa Rust, na sumusuporta sa Windows (x64 MSVC) at Android (ARM64). Saklaw ng gabay na ito ang toolchain, setup ng environment, at lahat ng kinakailangang command para makabuo ng mga gumaganang artifact.

> **Mga suportadong target lang:** Hindi sinusuportahan ang mga Linux at Windows GNU/MinGW na target. Magha-hard fail agad ang `build.rs` sa mga target na iyon na may malinaw na mensaheng nagtuturo sa tamang mga command.

> [!NOTE]
> Ang buong release pipeline —kabilang ang Windows installer at mga Cellar build— ay nasa [`.github/workflows/test_build.yml`](../.github/workflows/test_build.yml), ang awtoritatibong sanggunian kung paano ginagawa ang mga opisyal na artifact.

---

## 1. Pagkuha ng Source

```bash
git clone https://github.com/Tenshou170/Hachimi-Edge.git
cd Hachimi-Edge
```

Kung may mga hindi na-commit na pagbabago sa git working tree, magdaragdag ang Zygisk build ng `-dirty` na suffix sa bersyon ng module. Itakda ang `HACHIMI_IGNORE_DIRTY=true` para hindi ito isama (pinipin ng CI ang `false` para makita sa mga artifact ang mga lokal na pagbabago).

---

## 2. Mga Kailangan

### Rust Toolchain
I-install ang pinakabagong stable na Rust toolchain sa pamamagitan ng [rustup.rs](https://rustup.rs/).

### Windows (x64 MSVC)
- **Sa Windows:** Standard na MSVC toolchain, na naka-install sa pamamagitan ng Visual Studio o ng standalone na Build Tools.
- **Sa Linux / macOS:** `cargo-xwin`, na awtomatikong nagda-download at nagse-setup ng MSVC sysroot:
  ```bash
  cargo install cargo-xwin
  ```
  Kailangan din ang LLVM/Clang/LLD toolchain para sa link stage; siguraduhing available ang `llvm-lib` (sa CI ito ay symlink sa `llvm-ar`):
  ```bash
  # Debian / Ubuntu (LLVM 19, gaya sa CI)
  sudo apt-get install -y clang-19 lld-19 llvm-19
  sudo ln -sf /usr/lib/llvm-19/bin/llvm-ar /usr/local/bin/llvm-lib

  # macOS
  brew install llvm
  sudo ln -sf "$(brew --prefix llvm)/bin/llvm-ar" /usr/local/bin/llvm-lib
  ```

### Android (ARM64)
- Inirerekomenda ang **Android NDK r27d LTS**.
- Idagdag ang ARM64 na Rust target:
  ```bash
  rustup target add aarch64-linux-android
  ```
- Kailangan ang `zip` sa host kapag nagpopackage ng Zygisk module.

---

## 3. NDK Setup

Awtomatikong nadedetect ng build system ang NDK path, ayon sa ganitong pagkakasunod:

1. Environment variable na `$ANDROID_NDK_ROOT`
2. Environment variable na `$ANDROID_NDK_HOME`
3. Isang `./ndk` na symlink o directory sa project root

**Opsyon A — Environment variable (inirerekomenda para sa CI at one-off builds):**
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
```

**Opsyon B — Lokal na symlink (inirerekomenda para sa pang-araw-araw na development):**
```bash
# Linux / macOS
ln -s /path/to/android-ndk-r27d ndk

# Windows (Command Prompt)
mklink /J ndk C:\path\to\android-ndk-r27d
```

Hindi na kailangang i-edit nang mano-mano ang `.cargo/config.toml` — hinahandle ng build system ang lahat ng host platform.

---

## 4. Pagbu-build

### Windows (x64 MSVC)

**Pag-check ng compiler:**
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

Awtomatikong nadadetect ng script ang host OS:
- Sa Windows: native na build gamit ang MSVC toolchain.
- Sa Linux / macOS: cross-compile gamit ang `cargo-xwin`.

**Output:** `build/hachimi.dll`, pati na rin ang `build/blake3.json` (ginagawa kapag naka-install ang `b3sum`).

### Android (ARM64)

**Pag-check ng compiler:**
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

Nagbu-build laban sa **API level 24** na may **16 KB page-size alignment**, na buong komportable mula Android 7.0 hanggang Android 15+.

**Output:** `build/libmain-arm64-v8a.so`, pati na rin ang `build/sha256.json` (ginagawa kapag may `sha256sum` o `shasum`).

### Zygisk Module (Android)

```bash
RELEASE=1 ./tools/android/build_zygisk.sh
```

Pinapatakbo nito ang Android release build, tinatanggal ang debug symbols gamit ang `llvm-strip` ng NDK, nagpa-package ng module mula sa `tools/android/zygisk-template`, at sumusulat ng per-file SHA-256 checksums.

**Output:** `build/zygisk-hachimi-edge-v<bersyon>-<commit>[-dirty]-release.zip` (module na nai-install sa Magisk/KernelSU). Nangangailangan ng `zip` sa host.

### Mga Karagdagang Cargo Argument

Parehong ipinapasa ng mga build script ang mga karagdagang argumento sa cargo sa pamamagitan ng `CARGOARGS`:

```bash
RELEASE=1 CARGOARGS="--features some_feature" ./tools/windows/build.sh
```

---

## 5. Sanggunian sa mga Cargo Alias

Nakadefine sa `.cargo/config.toml`. Ang mga alias ng `check`, `clippy`, at `test` ay eksaktong kumokopya sa mga command sa CI, kabilang ang `-D warnings`, kaya ang lokal na pagtakbo ay kapareho ng CI. Para sa paggawa ng mga release artifact, gamitin ang mga build script sa itaas sa halip na mga build alias.

| Alias | Paglalarawan |
|---|---|
| `cargo xcheck` | Compiler check para sa Windows MSVC target (release profile) |
| `cargo xbuild` | Build para sa Windows MSVC gamit ang `cargo-xwin` (release profile) |
| `cargo xclippy` | Clippy lint para sa Windows MSVC target (`-D warnings`) |
| `cargo xtest` | Pag-compile ng mga test para sa Windows MSVC target (`--no-run`) |
| `cargo acheck` | Compiler check para sa Android ARM64 target (release profile) |
| `cargo abuild` | Build para sa Android ARM64 (release profile) |
| `cargo aclippy` | Clippy lint para sa Android ARM64 target (`-D warnings`) |
| `cargo atest` | Pag-compile ng mga test para sa Android ARM64 target (`--no-run`) |
