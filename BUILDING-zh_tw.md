# Hachimi Edge 構建指南

Hachimi Edge 是一個使用 Rust 編寫的跨平台遊戲增強與翻譯模組，支援 Windows (x64) 和 Android (ARM64)。

---

## 1. 前置要求

### Rust 工具鏈
請通過 [rustup.rs](https://rustup.rs/) 安裝最新的穩定版 Rust 工具鏈。

### Windows 支援
- **原生構建 (在 Windows 上)**: 標準 MSVC 工具鏈（通過 Visual Studio 或 Build Tools 安裝）。
- **交叉編譯 (在 Linux 上)**: 需要在 Linux 主機上安裝 `cargo-xwin` 以構建 Windows 版本：
  ```bash
  cargo install cargo-xwin
  ```

### Android 支援
- **Android NDK**: 強烈建議使用 **r27d LTS** (長期支援版本)。
- **Rust Target**: 添加 ARM64 Android 交叉編譯目標：
  ```bash
  rustup target add aarch64-linux-android
  ```

---

## 2. 依賴項配置 (實現界面視覺一致性)

為了與官方發布的構建版本（例如自定義 Combo Box 大小和 UI 渲染）保持絕對的視覺一致性，Hachimi Edge 需要針對自定義修補版本的 Egui 進行編譯。
此過程**完全由 Cargo 自動處理**。`Cargo.toml` 中已預先配置好 Git 補丁（指向 Git Fork 倉庫 `THShafi170/egui` 的 `hachimi-patches` 分支）。在編譯項目时，Cargo 會自動拉取、緩存並應用這些修改，無需開發人員進行任何手動克隆、修補或運行本地設置腳本！

---

## 3. 本地 NDK 環境配置

為了保持倉庫的整潔並避免硬編碼絕對路徑，Hachimi 的構建配置使用項目根目錄下名為 `ndk` 的符號連結，指向您的 Android NDK 目錄。

### 在 Linux / macOS 上：
創建指向您已解壓的 NDK 資料夾（例如 `r27d`）的符號連結：
```bash
ln -s /home/user/ndk/android-ndk-r27d ndk
```

### 在 Windows 上：
使用命令提示字元或 PowerShell 創建目錄聯結：
```cmd
mklink /J ndk C:\path\to\android-ndk-r27d
```

*注意：`ndk` 連結已被 Git 自動忽略。*

---

## 4. 編譯模組

### Windows (x64)

#### 在 Windows 上構建：
```bash
cargo build --target x86_64-pc-windows-msvc --release
```

#### 在 Linux 上交叉編譯：
使用我們預先配置的 Cargo 別名（它會調用 `cargo-xwin`）：
```bash
cargo xbuild
```

**構建產物**：`target/x86_64-pc-windows-msvc/release/hachimi.dll`

---

### Android (ARM64)

#### 本地構建：
運行我們預先配置的 Cargo 別名（需要步驟 3 中設置的 NDK 符號連結）：
```bash
cargo abuild
```
*注意：這將使用統一的 **API 級別 24** 進行構建，並以 **16KB 記憶體頁面大小對齊** 為目標，從而保證了對 Android 7.0 及以上版本的完美向下相容性，以及對 Android 15 設備的高效向前相容性。*

#### 在本地使用 CI 腳本構建 (需要設置 `ANDROID_NDK_ROOT`):
如果您想運行與我們的 GitHub Actions 運行器完全相同的腳本：
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
RELEASE=1 ./tools/android/build.sh
```

**構建產物**：`target/aarch64-linux-android/release/libhachimi.so`

---

## 常用 Cargo 別名說明

這些別名在 `.cargo/config.toml` 中定義，用以規範開發人員的工作流程：

*   `cargo abuild`: 使用本地 `ndk` 符號連結在 release 模式下編譯 Android 版本。
*   `cargo acheck`: 用於 Android 目標的快速編譯器檢查。
*   `cargo xbuild`: 運行 `cargo xwin` 在 Linux 主機上交叉編譯 Windows 版本。
*   `cargo xcheck`: 用於 Windows 交叉編譯目標的快速編譯器檢查。
