# Hachimi Edge 構建指南

Hachimi Edge 是一個使用 Rust 編寫的跨平台遊戲增強與翻譯模組，支援 Windows (x64 MSVC) 和 Android (ARM64)。

> **僅支援特定目標平台：** 不支援 Linux 或 Windows GNU/MinGW 目標。若嘗試使用這些目標，`build.rs` 會立即報錯並提示正確的構建命令。

---

## 1. 前置要求

### Rust 工具鏈
請通過 [rustup.rs](https://rustup.rs/) 安裝最新的穩定版 Rust 工具鏈。

### Windows (x64 MSVC)
- **在 Windows 上：** 標準 MSVC 工具鏈（通過 Visual Studio 或獨立的 Build Tools 安裝）。
- **在 Linux / macOS 上：** 需要 `cargo-xwin`，它會自動下載並配置 MSVC sysroot：
  ```bash
  cargo install cargo-xwin
  ```

### Android (ARM64)
- 推薦使用 **Android NDK r27d LTS**。
- 添加 ARM64 Rust 目標：
  ```bash
  rustup target add aarch64-linux-android
  ```

---

## 2. NDK 環境配置

構建系統會按以下優先級自動查找 NDK 路徑：

1. `$ANDROID_NDK_ROOT` 環境變數
2. `$ANDROID_NDK_HOME` 環境變數
3. 專案根目錄下名為 `ndk` 的符號連結或目錄

**方式 A — 環境變數（推薦用於 CI 及臨時構建）：**
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
```

**方式 B — 本地符號連結（推薦用於日常開發）：**
```bash
# Linux / macOS
ln -s /path/to/android-ndk-r27d ndk

# Windows（命令提示字元）
mklink /J ndk C:\path\to\android-ndk-r27d
```

無需手動修改 `.cargo/config.toml`——構建系統會自動處理所有主機平台。

---

## 3. 編譯模組

### Windows (x64 MSVC)

**編譯器檢查：**
```bash
cargo xcheck
```

**偵錯構建：**
```bash
./tools/windows/build.sh
```

**發布構建：**
```bash
RELEASE=1 ./tools/windows/build.sh
```

腳本會自動偵測主機作業系統：
- 在 Windows 上：直接使用 MSVC 工具鏈進行原生構建。
- 在 Linux / macOS 上：通過 `cargo-xwin` 進行交叉編譯。

**構建產物：** `build/hachimi.dll` 及 `build/blake3.json`（僅發布構建）

---

### Android (ARM64)

**編譯器檢查：**
```bash
cargo acheck
```

**偵錯構建：**
```bash
./tools/android/build.sh
```

**發布構建：**
```bash
RELEASE=1 ./tools/android/build.sh
```

使用 **API 級別 24** 及 **16KB 記憶體頁面大小對齊** 進行構建，完全相容 Android 7.0 至 Android 15+。

**構建產物：** `build/libmain-arm64-v8a.so` 及 `build/sha256.json`（僅發布構建）

---

## 4. Cargo 別名參考

定義於 `.cargo/config.toml`，僅用於日常開發——發布產物請使用上述構建腳本。

| 別名 | 說明 |
|---|---|
| `cargo xcheck` | Windows MSVC 目標編譯器檢查（release 模式） |
| `cargo xbuild` | 構建 Windows MSVC（不含 `--release`；發布時請使用腳本） |
| `cargo xclippy` | Windows MSVC 目標 Clippy 檢查 |
| `cargo acheck` | Android ARM64 目標編譯器檢查（release 模式） |
| `cargo abuild` | 構建 Android ARM64（不含 `--release`；發布時請使用腳本） |
| `cargo aclippy` | Android ARM64 目標 Clippy 檢查 |
