# Hachimi Edge 构建指南

Hachimi Edge 是一个使用 Rust 编写的跨平台游戏增强与翻译模组，支持 Windows (x64 MSVC) 和 Android (ARM64)。

> **仅支持特定目标平台：** 不支持 Linux 或 Windows GNU/MinGW 目标。若尝试使用这些目标，`build.rs` 会立即报错并提示正确的构建命令。

---

## 1. 前置要求

### Rust 工具链
请通过 [rustup.rs](https://rustup.rs/) 安装最新的稳定版 Rust 工具链。

### Windows (x64 MSVC)
- **在 Windows 上：** 标准 MSVC 工具链（通过 Visual Studio 或独立的 Build Tools 安装）。
- **在 Linux / macOS 上：** 需要 `cargo-xwin`，它会自动下载并配置 MSVC sysroot：
  ```bash
  cargo install cargo-xwin
  ```

### Android (ARM64)
- 推荐使用 **Android NDK r27d LTS**。
- 添加 ARM64 Rust 目标：
  ```bash
  rustup target add aarch64-linux-android
  ```

---

## 2. NDK 环境配置

构建系统会按以下优先级自动查找 NDK 路径：

1. `$ANDROID_NDK_ROOT` 环境变量
2. `$ANDROID_NDK_HOME` 环境变量
3. 项目根目录下名为 `ndk` 的符号链接或目录

**方式 A — 环境变量（推荐用于 CI 及临时构建）：**
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
```

**方式 B — 本地符号链接（推荐用于日常开发）：**
```bash
# Linux / macOS
ln -s /path/to/android-ndk-r27d ndk

# Windows（命令提示符）
mklink /J ndk C:\path\to\android-ndk-r27d
```

无需手动修改 `.cargo/config.toml`——构建系统会自动处理所有主机平台。

---

## 3. 编译模组

### Windows (x64 MSVC)

**编译器检查：**
```bash
cargo xcheck
```

**调试构建：**
```bash
./tools/windows/build.sh
```

**发布构建：**
```bash
RELEASE=1 ./tools/windows/build.sh
```

脚本会自动检测主机操作系统：
- 在 Windows 上：直接使用 MSVC 工具链进行原生构建。
- 在 Linux / macOS 上：通过 `cargo-xwin` 进行交叉编译。

**构建产物：** `build/hachimi.dll` 及 `build/blake3.json`（仅发布构建）

---

### Android (ARM64)

**编译器检查：**
```bash
cargo acheck
```

**调试构建：**
```bash
./tools/android/build.sh
```

**发布构建：**
```bash
RELEASE=1 ./tools/android/build.sh
```

使用 **API 级别 24** 及 **16KB 内存页面大小对齐** 进行构建，完全兼容 Android 7.0 至 Android 15+。

**构建产物：** `build/libmain-arm64-v8a.so` 及 `build/sha256.json`（仅发布构建）

---

## 4. Cargo 别名参考

定义于 `.cargo/config.toml`，仅用于日常开发——发布产物请使用上述构建脚本。

| 别名 | 说明 |
|---|---|
| `cargo xcheck` | Windows MSVC 目标编译器检查（release 模式） |
| `cargo xbuild` | 构建 Windows MSVC（不含 `--release`；发布时请使用脚本） |
| `cargo xclippy` | Windows MSVC 目标 Clippy 检查 |
| `cargo acheck` | Android ARM64 目标编译器检查（release 模式） |
| `cargo abuild` | 构建 Android ARM64（不含 `--release`；发布时请使用脚本） |
| `cargo aclippy` | Android ARM64 目标 Clippy 检查 |
