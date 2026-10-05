# Hachimi Edge 构建指南

Hachimi Edge 是一个使用 Rust 编写的跨平台游戏增强与翻译模组，支持 Windows (x64 MSVC) 和 Android (ARM64)。本指南涵盖工具链、环境配置以及生成可用构建产物所需的全部命令。

> **仅支持特定目标平台：** 不支持 Linux 或 Windows GNU/MinGW 目标。若尝试使用这些目标，`build.rs` 会立即报错并提示正确的构建命令。

> [!NOTE]
> 完整的发布流水线——包括 Windows 安装器与 Cellar 构建——位于 [`.github/workflows/test_build.yml`](../.github/workflows/test_build.yml)，这是官方构建产物生成方式的权威参考。

---

## 1. 获取源码

```bash
git clone https://github.com/Tenshou170/Hachimi-Edge.git
cd Hachimi-Edge
```

如果 git 工作区存在未提交的改动，Zygisk 构建会在模块版本号后附加 `-dirty` 后缀。设置 `HACHIMI_IGNORE_DIRTY=true` 可禁用该行为（CI 固定为 `false`，以便在构建产物中保留本地修改的标记）。

---

## 2. 前置要求

### Rust 工具链
请通过 [rustup.rs](https://rustup.rs/) 安装最新的稳定版 Rust 工具链。

### Windows (x64 MSVC)
- **在 Windows 上：** 标准 MSVC 工具链（通过 Visual Studio 或独立的 Build Tools 安装）。
- **在 Linux / macOS 上：** 需要 `cargo-xwin`，它会自动下载并配置 MSVC sysroot：
  ```bash
  cargo install cargo-xwin
  ```
  链接阶段还需要 LLVM/Clang/LLD 工具链；请确保 `llvm-lib` 可用（CI 中它是 `llvm-ar` 的符号链接）：
  ```bash
  # Debian / Ubuntu（CI 使用 LLVM 19）
  sudo apt-get install -y clang-19 lld-19 llvm-19
  sudo ln -sf /usr/lib/llvm-19/bin/llvm-ar /usr/local/bin/llvm-lib

  # macOS
  brew install llvm
  sudo ln -sf "$(brew --prefix llvm)/bin/llvm-ar" /usr/local/bin/llvm-lib
  ```

### Android (ARM64)
- 推荐使用 **Android NDK r27d LTS**。
- 添加 ARM64 Rust 目标：
  ```bash
  rustup target add aarch64-linux-android
  ```
- 打包 Zygisk 模块时，主机需要安装 `zip`。

---

## 3. NDK 环境配置

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

## 4. 编译模组

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

**构建产物：** `build/hachimi.dll`，以及 `build/blake3.json`（安装了 `b3sum` 时生成）。

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

**构建产物：** `build/libmain-arm64-v8a.so`，以及 `build/sha256.json`（有 `sha256sum` 或 `shasum` 时生成）。

### Zygisk 模块（Android）

```bash
RELEASE=1 ./tools/android/build_zygisk.sh
```

该脚本会执行 Android 发布构建，用 NDK 自带的 `llvm-strip` 去除调试符号，从 `tools/android/zygisk-template` 打包模块，并为每个文件写入 SHA-256 校验和。

**构建产物：** `build/zygisk-hachimi-edge-v<版本>-<提交>[-dirty]-release.zip`（可用于 Magisk/KernelSU 安装的模块）。主机需要安装 `zip`。

### 额外的 Cargo 参数

两个构建脚本都会通过 `CARGOARGS` 将额外参数转发给 cargo：

```bash
RELEASE=1 CARGOARGS="--features some_feature" ./tools/windows/build.sh
```

---

## 5. Cargo 别名参考

定义于 `.cargo/config.toml`。`check`、`clippy` 与 `test` 别名与 CI 中的命令逐字一致（包括 `-D warnings`），因此本地运行结果与 CI 完全相同。生成发布产物请使用上述构建脚本，而非构建别名。

| 别名 | 说明 |
|---|---|
| `cargo xcheck` | Windows MSVC 目标编译器检查（release 模式） |
| `cargo xbuild` | 通过 `cargo-xwin` 构建 Windows MSVC（release 模式） |
| `cargo xclippy` | Windows MSVC 目标 Clippy 检查（`-D warnings`） |
| `cargo xtest` | 编译 Windows MSVC 目标的测试（`--no-run`） |
| `cargo acheck` | Android ARM64 目标编译器检查（release 模式） |
| `cargo abuild` | 构建 Android ARM64（release 模式） |
| `cargo aclippy` | Android ARM64 目标 Clippy 检查（`-D warnings`） |
| `cargo atest` | 编译 Android ARM64 目标的测试（`--no-run`） |
