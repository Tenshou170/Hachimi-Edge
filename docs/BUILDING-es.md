# Guía de compilación de Hachimi Edge

Hachimi Edge es un mod de mejora y traducción de juegos multiplataforma escrito en Rust, compatible con Windows (x64 MSVC) y Android (ARM64). Esta guía cubre la cadena de herramientas, la configuración del entorno y todos los comandos necesarios para generar artefactos funcionales.

> **Solo objetivos compatibles:** Los objetivos Linux y Windows GNU/MinGW no están soportados. `build.rs` fallará inmediatamente para esos objetivos con un mensaje claro que apunta a los comandos correctos.

> [!NOTE]
> El pipeline completo de publicación —incluido el instalador de Windows y las compilaciones de Cellar— se encuentra en [`.github/workflows/test_build.yml`](../.github/workflows/test_build.yml), la referencia autoritativa sobre cómo se generan los artefactos oficiales.

---

## 1. Obtener el código fuente

```bash
git clone https://github.com/Tenshou170/Hachimi-Edge.git
cd Hachimi-Edge
```

Si el árbol de trabajo de git tiene cambios sin confirmar, la compilación de Zygisk añade un sufijo `-dirty` a la versión del módulo. Establece `HACHIMI_IGNORE_DIRTY=true` para suprimirlo (CI lo fija en `false` para que las modificaciones locales se reflejen en los artefactos).

---

## 2. Requisitos previos

### Cadena de herramientas de Rust
Instala la última versión estable de Rust a través de [rustup.rs](https://rustup.rs/).

### Windows (x64 MSVC)
- **En Windows:** la cadena de herramientas MSVC estándar, instalada mediante Visual Studio o las Build Tools independientes.
- **En Linux / macOS:** `cargo-xwin`, que descarga y configura el sysroot de MSVC automáticamente:
  ```bash
  cargo install cargo-xwin
  ```
  También se necesita la cadena de herramientas LLVM/Clang/LLD para la fase de enlazado; asegúrate de que `llvm-lib` esté disponible (en CI es un enlace simbólico a `llvm-ar`):
  ```bash
  # Debian / Ubuntu (LLVM 19, como en CI)
  sudo apt-get install -y clang-19 lld-19 llvm-19
  sudo ln -sf /usr/lib/llvm-19/bin/llvm-ar /usr/local/bin/llvm-lib

  # macOS
  brew install llvm
  sudo ln -sf "$(brew --prefix llvm)/bin/llvm-ar" /usr/local/bin/llvm-lib
  ```

### Android (ARM64)
- Se recomienda el **Android NDK r27d LTS**.
- Añade el objetivo ARM64 de Rust:
  ```bash
  rustup target add aarch64-linux-android
  ```
- Se requiere `zip` en el host al empaquetar el módulo Zygisk.

---

## 3. Configuración del NDK

El sistema de compilación descubre la ruta del NDK automáticamente, con este orden de prioridad:

1. Variable de entorno `$ANDROID_NDK_ROOT`
2. Variable de entorno `$ANDROID_NDK_HOME`
3. Un enlace simbólico o directorio `./ndk` en la raíz del proyecto

**Opción A — Variable de entorno (recomendado para CI y compilaciones puntuales):**
```bash
export ANDROID_NDK_ROOT=/ruta/a/android-ndk-r27d
```

**Opción B — Enlace simbólico local (recomendado para el desarrollo diario):**
```bash
# Linux / macOS
ln -s /ruta/a/android-ndk-r27d ndk

# Windows (Símbolo del sistema)
mklink /J ndk C:\ruta\a\android-ndk-r27d
```

No es necesario editar `.cargo/config.toml` manualmente: el sistema de compilación gestiona todos los hosts automáticamente.

---

## 4. Compilación

### Windows (x64 MSVC)

**Verificación del compilador:**
```bash
cargo xcheck
```

**Compilación de depuración:**
```bash
./tools/windows/build.sh
```

**Compilación de lanzamiento (release):**
```bash
RELEASE=1 ./tools/windows/build.sh
```

El script detecta el sistema operativo del host automáticamente:
- En Windows: compila de forma nativa con la cadena de herramientas MSVC.
- En Linux / macOS: compila de forma cruzada mediante `cargo-xwin`.

**Salida:** `build/hachimi.dll`, además de `build/blake3.json` (generado si `b3sum` está instalado).

### Android (ARM64)

**Verificación del compilador:**
```bash
cargo acheck
```

**Compilación de depuración:**
```bash
./tools/android/build.sh
```

**Compilación de lanzamiento (release):**
```bash
RELEASE=1 ./tools/android/build.sh
```

Compila contra el **nivel de API 24** con **alineación de páginas de 16 KB**, ofreciendo compatibilidad completa desde Android 7.0 hasta Android 15+.

**Salida:** `build/libmain-arm64-v8a.so`, además de `build/sha256.json` (generado si hay `sha256sum` o `shasum` disponible).

### Módulo Zygisk (Android)

```bash
RELEASE=1 ./tools/android/build_zygisk.sh
```

Ejecuta la compilación de lanzamiento de Android, elimina los símbolos de depuración con `llvm-strip` del NDK, empaqueta el módulo desde `tools/android/zygisk-template` y escribe sumas de verificación SHA-256 por archivo.

**Salida:** `build/zygisk-hachimi-edge-v<versión>-<commit>[-dirty]-release.zip` (módulo instalable en Magisk/KernelSU). Requiere `zip` en el host.

### Argumentos adicionales de Cargo

Ambos scripts de compilación reenvían argumentos adicionales a cargo mediante `CARGOARGS`:

```bash
RELEASE=1 CARGOARGS="--features some_feature" ./tools/windows/build.sh
```

---

## 5. Referencia de alias de Cargo

Definidos en `.cargo/config.toml`. Los alias de `check`, `clippy` y `test` reproducen los comandos de CI byte a byte, incluido `-D warnings`, por lo que una ejecución local equivale a CI. Para generar artefactos de lanzamiento, usa los scripts de compilación anteriores en lugar de los alias de compilación.

| Alias | Descripción |
|---|---|
| `cargo xcheck` | Verificación del compilador para el objetivo Windows MSVC (perfil release) |
| `cargo xbuild` | Compilación para Windows MSVC mediante `cargo-xwin` (perfil release) |
| `cargo xclippy` | Lint de Clippy para el objetivo Windows MSVC (`-D warnings`) |
| `cargo xtest` | Compilación de pruebas para el objetivo Windows MSVC (`--no-run`) |
| `cargo acheck` | Verificación del compilador para el objetivo Android ARM64 (perfil release) |
| `cargo abuild` | Compilación para Android ARM64 (perfil release) |
| `cargo aclippy` | Lint de Clippy para el objetivo Android ARM64 (`-D warnings`) |
| `cargo atest` | Compilación de pruebas para el objetivo Android ARM64 (`--no-run`) |
