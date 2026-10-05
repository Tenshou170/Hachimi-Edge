# Hachimi Edge 빌드 가이드

Hachimi Edge는 Rust로 작성된 크로스 플랫폼 게임 개선 및 번역 모드로, Windows(x64 MSVC)와 Android(ARM64)를 지원합니다. 이 가이드는 툴체인, 환경 설정, 동작하는 아티팩트를 만드는 데 필요한 모든 명령을 다룹니다.

> **지원 대상 한정:** Linux 및 Windows GNU/MinGW 대상은 지원되지 않습니다. 이 대상들을 사용하려 하면 `build.rs`가 올바른 명령을 알려주는 명확한 오류 메시지와 함께 즉시 실패합니다.

> [!NOTE]
> Windows 설치 프로그램과 Cellar 빌드를 포함한 전체 릴리스 파이프라인은 [`.github/workflows/test_build.yml`](../.github/workflows/test_build.yml)에 있으며, 이것이 공식 아티팩트가 생성되는 방식의 권위 있는 참고 자료입니다.

---

## 1. 소스 가져오기

```bash
git clone https://github.com/Tenshou170/Hachimi-Edge.git
cd Hachimi-Edge
```

git 작업 트리에 커밋되지 않은 변경 사항이 있으면 Zygisk 빌드는 모듈 버전에 `-dirty` 접미사를 붙입니다. 이를 숨기려면 `HACHIMI_IGNORE_DIRTY=true`로 설정하세요(CI는 `false`로 고정하여 로컬 수정 사항이 아티팩트에 표시되도록 합니다).

---

## 2. 사전 준비

### Rust 툴체인
[rustup.rs](https://rustup.rs/)를 통해 최신 안정版 Rust 툴체인을 설치하세요.

### Windows (x64 MSVC)
- **Windows에서:** Visual Studio 또는 독립 실행형 Build Tools로 설치하는 표준 MSVC 툴체인.
- **Linux / macOS에서:** `cargo-xwin`. MSVC sysroot를 자동으로 다운로드하고 설정합니다:
  ```bash
  cargo install cargo-xwin
  ```
  링크 단계에는 LLVM/Clang/LLD 툴체인도 필요합니다. `llvm-lib`을 사용할 수 있는지 확인하세요(CI에서는 `llvm-ar`에 대한 심볼릭 링크입니다):
  ```bash
  # Debian / Ubuntu (CI와 동일한 LLVM 19)
  sudo apt-get install -y clang-19 lld-19 llvm-19
  sudo ln -sf /usr/lib/llvm-19/bin/llvm-ar /usr/local/bin/llvm-lib

  # macOS
  brew install llvm
  sudo ln -sf "$(brew --prefix llvm)/bin/llvm-ar" /usr/local/bin/llvm-lib
  ```

### Android (ARM64)
- **Android NDK r27d LTS**를 권장합니다.
- ARM64 Rust 대상을 추가합니다:
  ```bash
  rustup target add aarch64-linux-android
  ```
- Zygisk 모듈을 패키징할 때는 호스트에 `zip`이 필요합니다.

---

## 3. NDK 설정

빌드 시스템은 다음 우선순위로 NDK 경로를 자동으로 찾습니다:

1. `$ANDROID_NDK_ROOT` 환경 변수
2. `$ANDROID_NDK_HOME` 환경 변수
3. 프로젝트 루트의 `./ndk` 심볼릭 링크 또는 디렉터리

**방법 A — 환경 변수(CI 및 일회성 빌드에 권장):**
```bash
export ANDROID_NDK_ROOT=/path/to/android-ndk-r27d
```

**방법 B — 로컬 심볼릭 링크(일상적인 개발에 권장):**
```bash
# Linux / macOS
ln -s /path/to/android-ndk-r27d ndk

# Windows (명령 프롬프트)
mklink /J ndk C:\path\to\android-ndk-r27d
```

`.cargo/config.toml`을 수동으로 편집할 필요가 없습니다. 빌드 시스템이 모든 호스트 플랫폼을 자동으로 처리합니다.

---

## 4. 빌드

### Windows (x64 MSVC)

**컴파일러 검사:**
```bash
cargo xcheck
```

**디버그 빌드:**
```bash
./tools/windows/build.sh
```

**릴리스 빌드:**
```bash
RELEASE=1 ./tools/windows/build.sh
```

스크립트는 호스트 OS를 자동으로 감지합니다:
- Windows에서: MSVC 툴체인으로 네이티브 빌드합니다.
- Linux / macOS에서: `cargo-xwin`으로 크로스 컴파일합니다.

**출력:** `build/hachimi.dll`, 그리고 `build/blake3.json`(`b3sum`이 설치된 경우 생성).

### Android (ARM64)

**컴파일러 검사:**
```bash
cargo acheck
```

**디버그 빌드:**
```bash
./tools/android/build.sh
```

**릴리스 빌드:**
```bash
RELEASE=1 ./tools/android/build.sh
```

**API 레벨 24**와 **16KB 페이지 크기 정렬**로 빌드되어 Android 7.0부터 Android 15+까지 완전히 호환됩니다.

**출력:** `build/libmain-arm64-v8a.so`, 그리고 `build/sha256.json`(`sha256sum` 또는 `shasum`이 있는 경우 생성).

### Zygisk 모듈 (Android)

```bash
RELEASE=1 ./tools/android/build_zygisk.sh
```

Android 릴리스 빌드를 실행하고, NDK의 `llvm-strip`으로 디버그 심볼을 제거하고, `tools/android/zygisk-template`에서 모듈을 패키징하며, 파일별 SHA-256 체크섬을 작성합니다.

**출력:** `build/zygisk-hachimi-edge-v<버전>-<커밋>[-dirty]-release.zip`(Magisk/KernelSU에 설치 가능한 모듈). 호스트에 `zip`이 필요합니다.

### 추가 Cargo 인수

두 빌드 스크립트 모두 `CARGOARGS`를 통해 추가 인수를 cargo에 전달합니다:

```bash
RELEASE=1 CARGOARGS="--features some_feature" ./tools/windows/build.sh
```

---

## 5. Cargo 별칭 참고

`.cargo/config.toml`에 정의되어 있습니다. `check`, `clippy`, `test` 별칭은 `-D warnings`를 포함해 CI의 명령과 바이트 단위로 동일하여, 로컬 실행 결과가 CI와 정확히 일치합니다. 릴리스 아티팩트를 만들 때는 빌드 별칭 대신 위의 빌드 스크립트를 사용하세요.

| 별칭 | 설명 |
|---|---|
| `cargo xcheck` | Windows MSVC 대상 컴파일러 검사(release 프로필) |
| `cargo xbuild` | `cargo-xwin`을 통한 Windows MSVC 빌드(release 프로필) |
| `cargo xclippy` | Windows MSVC 대상 Clippy 린트(`-D warnings`) |
| `cargo xtest` | Windows MSVC 대상 테스트 컴파일(`--no-run`) |
| `cargo acheck` | Android ARM64 대상 컴파일러 검사(release 프로필) |
| `cargo abuild` | Android ARM64 빌드(release 프로필) |
| `cargo aclippy` | Android ARM64 대상 Clippy 린트(`-D warnings`) |
| `cargo atest` | Android ARM64 대상 테스트 컴파일(`--no-run`) |
