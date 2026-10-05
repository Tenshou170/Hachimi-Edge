# Hướng dẫn biên dịch Hachimi Edge

Hachimi Edge là mod cải thiện và dịch thuật game đa nền tảng viết bằng Rust, hỗ trợ Windows (x64 MSVC) và Android (ARM64). Hướng dẫn này bao gồm bộ công cụ, thiết lập môi trường và mọi lệnh cần thiết để tạo ra các sản phẩm build hoạt động được.

> **Chỉ hỗ trợ các đích được liệt kê:** Các đích Linux và Windows GNU/MinGW không được hỗ trợ. `build.rs` sẽ báo lỗi ngay lập tức với các đích đó kèm thông báo rõ ràng chỉ đến lệnh đúng.

> [!NOTE]
> Toàn bộ quy trình phát hành — bao gồm trình cài Windows và các bản build Cellar — nằm trong [`.github/workflows/test_build.yml`](../.github/workflows/test_build.yml), tài liệu tham chiếu chính thức về cách các sản phẩm chính thức được tạo ra.

---

## 1. Lấy mã nguồn

```bash
git clone https://github.com/Tenshou170/Hachimi-Edge.git
cd Hachimi-Edge
```

Nếu cây làm việc của git có thay đổi chưa commit, bản build Zygisk sẽ thêm hậu tố `-dirty` vào phiên bản module. Đặt `HACHIMI_IGNORE_DIRTY=true` để tắt hành vi này (CI cố định là `false` để các chỉnh sửa cục bộ hiện rõ trong sản phẩm build).

---

## 2. Yêu cầu trước

### Bộ công cụ Rust
Cài bộ công cụ Rust stable mới nhất qua [rustup.rs](https://rustup.rs/).

### Windows (x64 MSVC)
- **Trên Windows:** Bộ công cụ MSVC tiêu chuẩn, cài qua Visual Studio hoặc Build Tools độc lập.
- **Trên Linux / macOS:** `cargo-xwin`, tự tải về và thiết lập sysroot MSVC:
  ```bash
  cargo install cargo-xwin
  ```
  Bộ công cụ LLVM/Clang/LLD cũng cần cho bước liên kết; đảm bảo `llvm-lib` khả dụng (trên CI nó là symlink tới `llvm-ar`):
  ```bash
  # Debian / Ubuntu (LLVM 19, như trên CI)
  sudo apt-get install -y clang-19 lld-19 llvm-19
  sudo ln -sf /usr/lib/llvm-19/bin/llvm-ar /usr/local/bin/llvm-lib

  # macOS
  brew install llvm
  sudo ln -sf "$(brew --prefix llvm)/bin/llvm-ar" /usr/local/bin/llvm-lib
  ```

### Android (ARM64)
- Khuyến nghị **Android NDK r27d LTS**.
- Thêm đích ARM64 cho Rust:
  ```bash
  rustup target add aarch64-linux-android
  ```
- Cần `zip` trên máy khi đóng gói module Zygisk.

---

## 3. Thiết lập NDK

Hệ thống build tự động tìm đường dẫn NDK theo thứ tự ưu tiên sau:

1. Biến môi trường `$ANDROID_NDK_ROOT`
2. Biến môi trường `$ANDROID_NDK_HOME`
3. Symlink hoặc thư mục `./ndk` ở gốc dự án

**Cách A — Biến môi trường (khuyến nghị cho CI và build một lần):**
```bash
export ANDROID_NDK_ROOT=/đường/dẫn/android-ndk-r27d
```

**Cách B — Symlink cục bộ (khuyến nghị cho phát triển hằng ngày):**
```bash
# Linux / macOS
ln -s /đường/dẫn/android-ndk-r27d ndk

# Windows (Command Prompt)
mklink /J ndk C:\đường\dẫn\android-ndk-r27d
```

Không cần chỉnh `.cargo/config.toml` thủ công — hệ thống build tự xử lý mọi nền tảng máy chủ.

---

## 4. Biên dịch

### Windows (x64 MSVC)

**Kiểm tra trình biên dịch:**
```bash
cargo xcheck
```

**Build debug:**
```bash
./tools/windows/build.sh
```

**Build release:**
```bash
RELEASE=1 ./tools/windows/build.sh
```

Tập lệnh tự phát hiện hệ điều hành của máy:
- Trên Windows: build gốc bằng bộ công cụ MSVC.
- Trên Linux / macOS: biên dịch chéo qua `cargo-xwin`.

**Kết quả:** `build/hachimi.dll`, cùng với `build/blake3.json` (tạo khi `b3sum` đã cài).

### Android (ARM64)

**Kiểm tra trình biên dịch:**
```bash
cargo acheck
```

**Build debug:**
```bash
./tools/android/build.sh
```

**Build release:**
```bash
RELEASE=1 ./tools/android/build.sh
```

Build với **API level 24** và **căn chỉnh kích thước trang 16 KB**, tương thích hoàn toàn từ Android 7.0 đến Android 15+.

**Kết quả:** `build/libmain-arm64-v8a.so`, cùng với `build/sha256.json` (tạo khi có `sha256sum` hoặc `shasum`).

### Module Zygisk (Android)

```bash
RELEASE=1 ./tools/android/build_zygisk.sh
```

Chạy bản build release của Android, gỡ ký hiệu debug bằng `llvm-strip` của NDK, đóng gói module từ `tools/android/zygisk-template`, và ghi checksum SHA-256 cho từng tệp.

**Kết quả:** `build/zygisk-hachimi-edge-v<phiên bản>-<commit>[-dirty]-release.zip` (module cài được qua Magisk/KernelSU). Cần `zip` trên máy chủ.

### Tham số Cargo bổ sung

Cả hai tập lệnh build đều chuyển tiếp tham số bổ sung tới cargo qua `CARGOARGS`:

```bash
RELEASE=1 CARGOARGS="--features some_feature" ./tools/windows/build.sh
```

---

## 5. Tham chiếu alias Cargo

Được định nghĩa trong `.cargo/config.toml`. Các alias `check`, `clippy` và `test` sao chép chính xác lệnh trong CI, bao gồm `-D warnings`, nên chạy cục bộ cho kết quả y hệt CI. Để tạo sản phẩm release, hãy dùng các tập lệnh build ở trên thay vì alias build.

| Alias | Mô tả |
|---|---|
| `cargo xcheck` | Kiểm tra trình biên dịch cho đích Windows MSVC (profile release) |
| `cargo xbuild` | Build Windows MSVC qua `cargo-xwin` (profile release) |
| `cargo xclippy` | Lint Clippy cho đích Windows MSVC (`-D warnings`) |
| `cargo xtest` | Biên dịch kiểm thử cho đích Windows MSVC (`--no-run`) |
| `cargo acheck` | Kiểm tra trình biên dịch cho đích Android ARM64 (profile release) |
| `cargo abuild` | Build Android ARM64 (profile release) |
| `cargo aclippy` | Lint Clippy cho đích Android ARM64 (`-D warnings`) |
| `cargo atest` | Biên dịch kiểm thử cho đích Android ARM64 (`--no-run`) |
