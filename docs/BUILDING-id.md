# Panduan Membangun Hachimi Edge

Hachimi Edge adalah mod peningkatan dan terjemahan game lintas platform yang ditulis dalam Rust, mendukung Windows (x64 MSVC) dan Android (ARM64). Panduan ini mencakup toolchain, penyiapan lingkungan, dan semua perintah yang diperlukan untuk menghasilkan artefak yang berfungsi.

> **Hanya target yang didukung:** Target Linux dan Windows GNU/MinGW tidak didukung. `build.rs` akan langsung gagal untuk target tersebut dengan pesan kesalahan yang jelas mengarah ke perintah yang benar.

> [!NOTE]
> Seluruh pipeline rilis — termasuk penginstal Windows dan build Cellar — ada di [`.github/workflows/test_build.yml`](../.github/workflows/test_build.yml), referensi resmi untuk cara artefak resmi dibuat.

---

## 1. Mendapatkan Kode Sumber

```bash
git clone https://github.com/Tenshou170/Hachimi-Edge.git
cd Hachimi-Edge
```

Jika pohon kerja git memiliki perubahan yang belum dikomit, build Zygisk menambahkan akhiran `-dirty` ke versi modul. Tetapkan `HACHIMI_IGNORE_DIRTY=true` untuk menonaktifkannya (CI menguncinya ke `false` agar modifikasi lokal tetap terlihat di artefak).

---

## 2. Prasyarat

### Toolchain Rust
Instal toolchain Rust stable terbaru melalui [rustup.rs](https://rustup.rs/).

### Windows (x64 MSVC)
- **Di Windows:** toolchain MSVC standar, diinstal melalui Visual Studio atau Build Tools mandiri.
- **Di Linux / macOS:** `cargo-xwin`, yang mengunduh dan menyiapkan sysroot MSVC secara otomatis:
  ```bash
  cargo install cargo-xwin
  ```
  Toolchain LLVM/Clang/LLD juga diperlukan untuk tahap tautan; pastikan `llvm-lib` tersedia (di CI, ini adalah symlink ke `llvm-ar`):
  ```bash
  # Debian / Ubuntu (LLVM 19, seperti di CI)
  sudo apt-get install -y clang-19 lld-19 llvm-19
  sudo ln -sf /usr/lib/llvm-19/bin/llvm-ar /usr/local/bin/llvm-lib

  # macOS
  brew install llvm
  sudo ln -sf "$(brew --prefix llvm)/bin/llvm-ar" /usr/local/bin/llvm-lib
  ```

### Android (ARM64)
- **Android NDK r27d LTS** direkomendasikan.
- Tambahkan target ARM64 Rust:
  ```bash
  rustup target add aarch64-linux-android
  ```
- `zip` diperlukan pada host saat mengemas modul Zygisk.

---

## 3. Penyiapan NDK

Sistem build menemukan jalur NDK secara otomatis, dengan urutan prioritas berikut:

1. Variabel lingkungan `$ANDROID_NDK_ROOT`
2. Variabel lingkungan `$ANDROID_NDK_HOME`
3. Symlink atau direktori `./ndk` di root proyek

**Opsi A — Variabel lingkungan (direkomendasikan untuk CI dan build sekali jalan):**
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
```

**Opsi B — Symlink lokal (direkomendasikan untuk pengembangan sehari-hari):**
```bash
# Linux / macOS
ln -s /path/to/android-ndk-r27d ndk

# Windows (Command Prompt)
mklink /J ndk C:\path\to\android-ndk-r27d
```

Tidak perlu mengedit `.cargo/config.toml` secara manual — sistem build menangani semua platform host secara otomatis.

---

## 4. Membangun

### Windows (x64 MSVC)

**Pemeriksaan kompilator:**
```bash
cargo xcheck
```

**Build debug:**
```bash
./tools/windows/build.sh
```

**Build rilis:**
```bash
RELEASE=1 ./tools/windows/build.sh
```

Skrip mendeteksi OS host secara otomatis:
- Di Windows: membangun secara native dengan toolchain MSVC.
- Di Linux / macOS: kompilasi silang melalui `cargo-xwin`.

**Keluaran:** `build/hachimi.dll`, ditambah `build/blake3.json` (dibuat saat `b3sum` terinstal).

### Android (ARM64)

**Pemeriksaan kompilator:**
```bash
cargo acheck
```

**Build debug:**
```bash
./tools/android/build.sh
```

**Build rilis:**
```bash
RELEASE=1 ./tools/android/build.sh
```

Membangun dengan **API level 24** dan **penyelarasan ukuran halaman 16 KB**, memberikan kompatibilitas penuh dari Android 7.0 hingga Android 15+.

**Keluaran:** `build/libmain-arm64-v8a.so`, ditambah `build/sha256.json` (dibuat saat `sha256sum` atau `shasum` tersedia).

### Modul Zygisk (Android)

```bash
RELEASE=1 ./tools/android/build_zygisk.sh
```

Menjalankan build rilis Android, menghapus simbol debug dengan `llvm-strip` milik NDK, mengemas modul dari `tools/android/zygisk-template`, dan menulis checksum SHA-256 untuk setiap berkas.

**Keluaran:** `build/zygisk-hachimi-edge-v<versi>-<commit>[-dirty]-release.zip` (modul yang dapat diinstal di Magisk/KernelSU). Memerlukan `zip` pada host.

### Argumen Cargo Tambahan

Kedua skrip build meneruskan argumen tambahan ke cargo melalui `CARGOARGS`:

```bash
RELEASE=1 CARGOARGS="--features some_feature" ./tools/windows/build.sh
```

---

## 5. Referensi Alias Cargo

Didefinisikan di `.cargo/config.toml`. Alias `check`, `clippy`, dan `test` mencerminkan perintah di CI secara persis, termasuk `-D warnings`, sehingga eksekusi lokal sama persis dengan CI. Untuk menghasilkan artefak rilis, gunakan skrip build di atas alih-alih alias build.

| Alias | Deskripsi |
|---|---|
| `cargo xcheck` | Pemeriksaan kompilator untuk target Windows MSVC (profil release) |
| `cargo xbuild` | Build untuk Windows MSVC melalui `cargo-xwin` (profil release) |
| `cargo xclippy` | Lint Clippy untuk target Windows MSVC (`-D warnings`) |
| `cargo xtest` | Kompilasi pengujian untuk target Windows MSVC (`--no-run`) |
| `cargo acheck` | Pemeriksaan kompilator untuk target Android ARM64 (profil release) |
| `cargo abuild` | Build untuk Android ARM64 (profil release) |
| `cargo aclippy` | Lint Clippy untuk target Android ARM64 (`-D warnings`) |
| `cargo atest` | Kompilasi pengujian untuk target Android ARM64 (`--no-run`) |
