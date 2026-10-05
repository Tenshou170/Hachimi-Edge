# Руководство по сборке Hachimi Edge

Hachimi Edge — кроссплатформенный мод улучшения и перевода игры, написанный на Rust и поддерживающий Windows (x64 MSVC) и Android (ARM64). Это руководство охватывает инструментарий, настройку окружения и все команды, необходимые для получения работающих артефактов.

> **Только поддерживаемые цели:** Цели Linux и Windows GNU/MinGW не поддерживаются. `build.rs` немедленно завершится ошибкой для этих целей с понятным сообщением, указывающим правильные команды.

> [!NOTE]
> Полный конвейер выпуска — включая установщик Windows и сборки Cellar — находится в [`.github/workflows/test_build.yml`](../.github/workflows/test_build.yml), это авторитетный справочник по созданию официальных артефактов.

---

## 1. Получение исходников

```bash
git clone https://github.com/Tenshou170/Hachimi-Edge.git
cd Hachimi-Edge
```

Если в рабочем дереве git есть незакоммиченные изменения, сборка Zygisk добавит суффикс `-dirty` к версии модуля. Задайте `HACHIMI_IGNORE_DIRTY=true`, чтобы отключить это (в CI зафиксировано `false`, чтобы локальные изменения были видны в артефактах).

---

## 2. Необходимые компоненты

### Инструментарий Rust
Установите последнюю стабильную версию Rust через [rustup.rs](https://rustup.rs/).

### Windows (x64 MSVC)
- **На Windows:** стандартный инструментарий MSVC, устанавливаемый через Visual Studio или отдельные Build Tools.
- **На Linux / macOS:** `cargo-xwin`, который автоматически скачивает и настраивает sysroot MSVC:
  ```bash
  cargo install cargo-xwin
  ```
  Для этапа компоновки также нужны LLVM/Clang/LLD; убедитесь, что `llvm-lib` доступен (в CI это символическая ссылка на `llvm-ar`):
  ```bash
  # Debian / Ubuntu (LLVM 19, как в CI)
  sudo apt-get install -y clang-19 lld-19 llvm-19
  sudo ln -sf /usr/lib/llvm-19/bin/llvm-ar /usr/local/bin/llvm-lib

  # macOS
  brew install llvm
  sudo ln -sf "$(brew --prefix llvm)/bin/llvm-ar" /usr/local/bin/llvm-lib
  ```

### Android (ARM64)
- Рекомендуется **Android NDK r27d LTS**.
- Добавьте цель ARM64 в Rust:
  ```bash
  rustup target add aarch64-linux-android
  ```
- Для упаковки Zygisk-модуля на хосте требуется `zip`.

---

## 3. Настройка NDK

Система сборки находит путь к NDK автоматически в следующем порядке приоритета:

1. Переменная окружения `$ANDROID_NDK_ROOT`
2. Переменная окружения `$ANDROID_NDK_HOME`
3. Символическая ссылка или каталог `./ndk` в корне проекта

**Вариант A — переменная окружения (рекомендуется для CI и разовых сборок):**
```bash
export ANDROID_NDK_ROOT=/путь/к/android-ndk-r27d
```

**Вариант B — локальная символическая ссылка (рекомендуется для повседневной разработки):**
```bash
# Linux / macOS
ln -s /путь/к/android-ndk-r27d ndk

# Windows (командная строка)
mklink /J ndk C:\путь\к\android-ndk-r27d
```

Ручные правки `.cargo/config.toml` не нужны — система сборки сама учитывает все хост-платформы.

---

## 4. Сборка

### Windows (x64 MSVC)

**Проверка компилятором:**
```bash
cargo xcheck
```

**Отладочная сборка:**
```bash
./tools/windows/build.sh
```

**Релизная сборка:**
```bash
RELEASE=1 ./tools/windows/build.sh
```

Скрипт автоматически определяет ОС хоста:
- На Windows: нативная сборка инструментариеm MSVC.
- На Linux / macOS: кросс-компиляция через `cargo-xwin`.

**Результат:** `build/hachimi.dll`, а также `build/blake3.json` (создаётся, если установлен `b3sum`).

### Android (ARM64)

**Проверка компилятором:**
```bash
cargo acheck
```

**Отладочная сборка:**
```bash
./tools/android/build.sh
```

**Релизная сборка:**
```bash
RELEASE=1 ./tools/android/build.sh
```

Сборка выполняется для **API уровня 24** с **выравниванием по страницам 16 КБ**, что обеспечивает полную совместимость от Android 7.0 до Android 15+.

**Результат:** `build/libmain-arm64-v8a.so`, а также `build/sha256.json` (создаётся при наличии `sha256sum` или `shasum`).

### Zygisk-модуль (Android)

```bash
RELEASE=1 ./tools/android/build_zygisk.sh
```

Запускает релизную сборку Android, убирает отладочные символы через `llvm-strip` из NDK, упаковывает модуль из `tools/android/zygisk-template` и записывает SHA-256 контрольные суммы для каждого файла.

**Результат:** `build/zygisk-hachimi-edge-v<версия>-<коммит>[-dirty]-release.zip` (модуль, устанавливаемый через Magisk/KernelSU). Требуется `zip` на хосте.

### Дополнительные аргументы Cargo

Оба скрипта сборки передают дополнительные аргументы в cargo через `CARGOARGS`:

```bash
RELEASE=1 CARGOARGS="--features some_feature" ./tools/windows/build.sh
```

---

## 5. Справочник псевдонимов Cargo

Определены в `.cargo/config.toml`. Псевдонимы `check`, `clippy` и `test` побайтово повторяют команды CI, включая `-D warnings`, поэтому локальный запуск в точности воспроизводит CI. Для создания релизных артефактов используйте скрипты сборки выше, а не псевдонимы сборки.

| Псевдоним | Описание |
|---|---|
| `cargo xcheck` | Проверка компилятором для цели Windows MSVC (профиль release) |
| `cargo xbuild` | Сборка Windows MSVC через `cargo-xwin` (профиль release) |
| `cargo xclippy` | Линт Clippy для цели Windows MSVC (`-D warnings`) |
| `cargo xtest` | Компиляция тестов для цели Windows MSVC (`--no-run`) |
| `cargo acheck` | Проверка компилятором для цели Android ARM64 (профиль release) |
| `cargo abuild` | Сборка Android ARM64 (профиль release) |
| `cargo aclippy` | Линт Clippy для цели Android ARM64 (`-D warnings`) |
| `cargo atest` | Компиляция тестов для цели Android ARM64 (`--no-run`) |
