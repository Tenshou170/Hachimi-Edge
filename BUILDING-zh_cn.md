# Hachimi Edge 构建指南

Hachimi Edge 是一个使用 Rust 编写的跨平台游戏增强与翻译模组，支持 Windows (x64) 和 Android (ARM64)。

---

## 1. 前置要求

### Rust 工具链
请通过 [rustup.rs](https://rustup.rs/) 安装最新的稳定版 Rust 工具链。

### Windows 支持
- **原生构建 (在 Windows 上)**: 标准 MSVC 工具链（通过 Visual Studio 或 Build Tools 安装）。
- **交叉编译 (在 Linux 上)**: 需要在 Linux 主机上安装 `cargo-xwin` 以构建 Windows 版本：
  ```bash
  cargo install cargo-xwin
  ```

### Android 支持
- **Android NDK**: 强烈建议使用 **r27d LTS** (长期支持版本)。
- **Rust Target**: 添加 ARM64 Android 交叉编译目标：
  ```bash
  rustup target add aarch64-linux-android
  ```

---

## 2. 依赖项配置 (实现界面视觉一致性)

为了与官方发布的构建版本（例如自定义 Combo Box 大小和 UI 渲染）保持绝对的视觉一致性，Hachimi Edge 需要针对自定义修补版本的 Egui 进行编译。

此过程**完全由 Cargo 自动处理**。`Cargo.toml` 中已预先配置好 Git 补丁（指向 Git Fork 仓库 `THShafi170/egui` 的 `hachimi-patches` 分支）。在编译项目时，Cargo 会自动拉取、缓存并应用这些修改，无需开发人员进行任何手动克隆、修补或运行本地设置脚本！

---

## 3. 本地 NDK 环境配置

为了保持仓库的整洁并避免硬编码绝对路径，Hachimi 的构建配置使用项目根目录下名为 `ndk` 的符号链接，指向您的 Android NDK 目录。

### 在 Linux / macOS 上：
创建指向您已解压的 NDK 文件夹（例如 `r27d`）的符号链接：
```bash
ln -s /home/user/ndk/android-ndk-r27d ndk
```

### 在 Windows 上：
使用命令提示符或 PowerShell 创建目录联接：
```cmd
mklink /J ndk C:\path\to\android-ndk-r27d
```

*注意：`ndk` 链接已被 Git 自动忽略。*

---

## 4. 编译模组

### Windows (x64)

#### 在 Windows 上构建：
```bash
cargo build --target x86_64-pc-windows-msvc --release
```

#### 在 Linux 上交叉编译：
使用我们预先配置的 Cargo 别名（它会调用 `cargo-xwin`）：
```bash
cargo xbuild
```

**构建产物**：`target/x86_64-pc-windows-msvc/release/hachimi.dll`

---

### Android (ARM64)

#### 本地构建：
运行我们预先配置的 Cargo 别名（需要步骤 3 中设置的 NDK 符号链接）：
```bash
cargo abuild
```
*注意：这将使用统一的 **API 级别 24** 进行构建，并以 **16KB 内存页面大小对齐** 为目标，从而保证了对 Android 7.0 及以上版本的完美向下兼容性，以及对 Android 15 设备的高效向前兼容性。*

#### 在本地使用 CI 脚本构建 (需要设置 `ANDROID_NDK_ROOT`):
如果您想运行与我们的 GitHub Actions 运行器完全相同的脚本：
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
RELEASE=1 ./tools/android/build.sh
```

**构建产物**：`target/aarch64-linux-android/release/libhachimi.so`

---

## 常用 Cargo 别名说明

这些别名在 `.cargo/config.toml` 中定义，用以规范开发人员的工作流程：

*   `cargo abuild`: 使用本地 `ndk` 符号链接在 release 模式下编译 Android 版本。
*   `cargo acheck`: 用于 Android 目标的快速编译器检查。
*   `cargo xbuild`: 运行 `cargo xwin` 在 Linux 主机上交叉编译 Windows 版本。
*   `cargo xcheck`: 用于 Windows 交叉编译目标的快速编译器检查。
