# Hachimi Edge 構建指南

Hachimi Edge 是一個使用 Rust 編寫的跨平台遊戲增強與翻譯模組，支援 Windows (x64 MSVC) 和 Android (ARM64)。本指南涵蓋工具鏈、環境配置以及產生可用構建產物所需的全部命令。

> **僅支援特定目標平台：** 不支援 Linux 或 Windows GNU/MinGW 目標。若嘗試使用這些目標，`build.rs` 會立即報錯並提示正確的構建命令。

> [!NOTE]
> 完整的發布流水線——包括 Windows 安裝器與 Cellar 構建——位於 [`.github/workflows/test_build.yml`](../.github/workflows/test_build.yml)，這是官方構建產物產生方式的權威參考。

---

## 1. 取得原始碼

```bash
git clone https://github.com/Tenshou170/Hachimi-Edge.git
cd Hachimi-Edge
```

如果 git 工作區存在未提交的變更，Zygisk 構建會在模組版本號後附加 `-dirty` 後綴。設定 `HACHIMI_IGNORE_DIRTY=true` 可停用該行為（CI 固定為 `false`，以便在構建產物中保留本機修改的標記）。

---

## 2. 前置要求

### Rust 工具鏈
請通過 [rustup.rs](https://rustup.rs/) 安裝最新的穩定版 Rust 工具鏈。

### Windows (x64 MSVC)
- **在 Windows 上：** 標準 MSVC 工具鏈（通過 Visual Studio 或獨立的 Build Tools 安裝）。
- **在 Linux / macOS 上：** 需要 `cargo-xwin`，它會自動下載並配置 MSVC sysroot：
  ```bash
  cargo install cargo-xwin
  ```
  連結階段還需要 LLVM/Clang/LLD 工具鏈；請確保 `llvm-lib` 可用（CI 中它是 `llvm-ar` 的符號連結）：
  ```bash
  # Debian / Ubuntu（CI 使用 LLVM 19）
  sudo apt-get install -y clang-19 lld-19 llvm-19
  sudo ln -sf /usr/lib/llvm-19/bin/llvm-ar /usr/local/bin/llvm-lib

  # macOS
  brew install llvm
  sudo ln -sf "$(brew --prefix llvm)/bin/llvm-ar" /usr/local/bin/llvm-lib
  ```

### Android (ARM64)
- 推薦使用 **Android NDK r27d LTS**。
- 新增 ARM64 Rust 目標：
  ```bash
  rustup target add aarch64-linux-android
  ```
- 封裝 Zygisk 模組時，主機需要安裝 `zip`。

---

## 3. NDK 環境配置

構建系統會按以下優先級自動查找 NDK 路徑：

1. `$ANDROID_NDK_ROOT` 環境變數
2. `$ANDROID_NDK_HOME` 環境變數
3. 專案根目錄下名為 `ndk` 的符號連結或目錄

**方式 A — 環境變數（推薦用於 CI 及臨時構建）：**
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
```

**方式 B — 本機符號連結（推薦用於日常開發）：**
```bash
# Linux / macOS
ln -s /path/to/android-ndk-r27d ndk

# Windows（命令提示字元）
mklink /J ndk C:\path\to\android-ndk-r27d
```

無需手動修改 `.cargo/config.toml`——構建系統會自動處理所有主機平台。

---

## 4. 編譯模組

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

**構建產物：** `build/hachimi.dll`，以及 `build/blake3.json`（安裝了 `b3sum` 時產生）。

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

**構建產物：** `build/libmain-arm64-v8a.so`，以及 `build/sha256.json`（有 `sha256sum` 或 `shasum` 時產生）。

### Zygisk 模組（Android）

```bash
RELEASE=1 ./tools/android/build_zygisk.sh
```

該腳本會執行 Android 發布構建，用 NDK 內建的 `llvm-strip` 移除偵錯符號，從 `tools/android/zygisk-template` 封裝模組，並為每個檔案寫入 SHA-256 校驗和。

**構建產物：** `build/zygisk-hachimi-edge-v<版本>-<提交>[-dirty]-release.zip`（可用於 Magisk/KernelSU 安裝的模組）。主機需要安裝 `zip`。

### 額外的 Cargo 參數

兩個構建腳本都會透過 `CARGOARGS` 將額外參數轉發給 cargo：

```bash
RELEASE=1 CARGOARGS="--features some_feature" ./tools/windows/build.sh
```

---

## 5. Cargo 別名參考

定義於 `.cargo/config.toml`。`check`、`clippy` 與 `test` 別名與 CI 中的命令逐字一致（包括 `-D warnings`），因此本機執行結果與 CI 完全相同。產生發布產物請使用上述構建腳本，而非構建別名。

| 別名 | 說明 |
|---|---|
| `cargo xcheck` | Windows MSVC 目標編譯器檢查（release 模式） |
| `cargo xbuild` | 通過 `cargo-xwin` 構建 Windows MSVC（release 模式） |
| `cargo xclippy` | Windows MSVC 目標 Clippy 檢查（`-D warnings`） |
| `cargo xtest` | 編譯 Windows MSVC 目標的測試（`--no-run`） |
| `cargo acheck` | Android ARM64 目標編譯器檢查（release 模式） |
| `cargo abuild` | 構建 Android ARM64（release 模式） |
| `cargo aclippy` | Android ARM64 目標 Clippy 檢查（`-D warnings`） |
| `cargo atest` | 編譯 Android ARM64 目標的測試（`--no-run`） |
