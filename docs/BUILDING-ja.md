# Hachimi Edge ビルドガイド

Hachimi Edge は Rust で書かれたクロスプラットフォームのゲーム強化・翻訳 Mod で、Windows (x64 MSVC) と Android (ARM64) に対応しています。本ガイドでは、ツールチェーン、環境構築、動作する成果物を生成するために必要なすべてのコマンドを説明します。

> **対応ターゲットのみ：** Linux および Windows GNU/MinGW ターゲットは非対応です。これらのターゲットを指定すると、`build.rs` が正しいコマンドを案内する明確なエラーを出して即座に失敗します。

> [!NOTE]
> Windows インストーラーや Cellar のビルドを含む完全なリリースパイプラインは [`.github/workflows/test_build.yml`](../.github/workflows/test_build.yml) にあります。公式の成果物がどのように生成されるかの権威あるリファレンスです。

---

## 1. ソースの取得

```bash
git clone https://github.com/Tenshou170/Hachimi-Edge.git
cd Hachimi-Edge
```

git ワーキングツリーに未コミットの変更がある場合、Zygisk のビルドではモジュールのバージョンに `-dirty` 接尾辞が付きます。`HACHIMI_IGNORE_DIRTY=true` を設定するとこれを抑制できます（CI では `false` 固定で、ローカルの変更は成果物に反映されます）。

---

## 2. 事前に必要なもの

### Rust ツールチェーン
[rustup.rs](https://rustup.rs/) から最新の安定版 Rust ツールチェーンをインストールしてください。

### Windows (x64 MSVC)
- **Windows 上：** Visual Studio または単体の Build Tools でインストールする標準の MSVC ツールチェーン。
- **Linux / macOS 上：** `cargo-xwin`。MSVC sysroot を自動的にダウンロード・セットアップします：
  ```bash
  cargo install cargo-xwin
  ```
  リンク段階では LLVM/Clang/LLD ツールチェーンも必要です。`llvm-lib` が利用可能であることを確認してください（CI では `llvm-ar` へのシンボリックリンクです）：
  ```bash
  # Debian / Ubuntu（CI と同じく LLVM 19）
  sudo apt-get install -y clang-19 lld-19 llvm-19
  sudo ln -sf /usr/lib/llvm-19/bin/llvm-ar /usr/local/bin/llvm-lib

  # macOS
  brew install llvm
  sudo ln -sf "$(brew --prefix llvm)/bin/llvm-ar" /usr/local/bin/llvm-lib
  ```

### Android (ARM64)
- **Android NDK r27d LTS** を推奨します。
- ARM64 Rust ターゲットを追加します：
  ```bash
  rustup target add aarch64-linux-android
  ```
- Zygisk モジュールのパッケージングにはホスト側に `zip` が必要です。

---

## 3. NDK のセットアップ

ビルドシステムは次の優先順位で NDK のパスを自動検出します：

1. 環境変数 `$ANDROID_NDK_ROOT`
2. 環境変数 `$ANDROID_NDK_HOME`
3. プロジェクトルートにある `./ndk` のシンボリックリンクまたはディレクトリ

**方法 A — 環境変数（CI や単発ビルドに推奨）：**
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
```

**方法 B — ローカルのシンボリックリンク（日々の開発に推奨）：**
```bash
# Linux / macOS
ln -s /path/to/android-ndk-r27d ndk

# Windows（コマンドプロンプト）
mklink /J ndk C:\path\to\android-ndk-r27d
```

`.cargo/config.toml` を手動で編集する必要はありません。ビルドシステムがすべてのホストプラットフォームを自動的に処理します。

---

## 4. ビルド

### Windows (x64 MSVC)

**コンパイラチェック：**
```bash
cargo xcheck
```

**デバッグビルド：**
```bash
./tools/windows/build.sh
```

**リリースビルド：**
```bash
RELEASE=1 ./tools/windows/build.sh
```

スクリプトはホスト OS を自動判別します：
- Windows 上： MSVC ツールチェーンでネイティブにビルドします。
- Linux / macOS 上： `cargo-xwin` 経由でクロスコンパイルします。

**出力：** `build/hachimi.dll`、および `build/blake3.json`（`b3sum` がインストールされている場合に生成）。

### Android (ARM64)

**コンパイラチェック：**
```bash
cargo acheck
```

**デバッグビルド：**
```bash
./tools/android/build.sh
```

**リリースビルド：**
```bash
RELEASE=1 ./tools/android/build.sh
```

**API レベル 24** と **16 KB ページサイズアラインメント** でビルドされ、Android 7.0 から Android 15+ まで完全に互換性があります。

**出力：** `build/libmain-arm64-v8a.so`、および `build/sha256.json`（`sha256sum` または `shasum` がある場合に生成）。

### Zygisk モジュール（Android）

```bash
RELEASE=1 ./tools/android/build_zygisk.sh
```

Android のリリースビルドを実行し、NDK の `llvm-strip` でデバッグシンボルを除去、`tools/android/zygisk-template` からモジュールをパッケージングし、ファイルごとの SHA-256 チェックサムを書き出します。

**出力：** `build/zygisk-hachimi-edge-v<バージョン>-<コミット>[-dirty]-release.zip`（Magisk/KernelSU にインストール可能なモジュール）。ホストには `zip` が必要です。

### 追加の Cargo 引数

どちらのビルドスクリプトも、`CARGOARGS` 経由で追加の引数を cargo に渡します：

```bash
RELEASE=1 CARGOARGS="--features some_feature" ./tools/windows/build.sh
```

---

## 5. Cargo エイリアス一覧

`.cargo/config.toml` で定義されています。`check`・`clippy`・`test` のエイリアスは CI のコマンドを（`-D warnings` を含めて）バイト単位で再現するため、ローカルでの実行は CI と完全に一致します。リリース成果物の生成には、ビルド系エイリアスではなく上記のビルドスクリプトを使用してください。

| エイリアス | 説明 |
|---|---|
| `cargo xcheck` | Windows MSVC ターゲットのコンパイラチェック（release プロファイル） |
| `cargo xbuild` | `cargo-xwin` による Windows MSVC のビルド（release プロファイル） |
| `cargo xclippy` | Windows MSVC ターゲットの Clippy リント（`-D warnings`） |
| `cargo xtest` | Windows MSVC ターゲットのテストをコンパイル（`--no-run`） |
| `cargo acheck` | Android ARM64 ターゲットのコンパイラチェック（release プロファイル） |
| `cargo abuild` | Android ARM64 のビルド（release プロファイル） |
| `cargo aclippy` | Android ARM64 ターゲットの Clippy リント（`-D warnings`） |
| `cargo atest` | Android ARM64 ターゲットのテストをコンパイル（`--no-run`） |
