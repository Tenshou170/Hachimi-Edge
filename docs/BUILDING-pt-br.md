# Guia de compilação do Hachimi Edge

O Hachimi Edge é um mod de aprimoramento e tradução de jogos multiplataforma escrito em Rust, com suporte a Windows (x64 MSVC) e Android (ARM64). Este guia cobre a toolchain, a configuração do ambiente e todos os comandos necessários para produzir artefatos funcionais.

> **Apenas destinos suportados:** Os destinos Linux e Windows GNU/MinGW não são suportados. O `build.rs` falhará imediatamente para esses destinos com uma mensagem clara apontando para os comandos corretos.

> [!NOTE]
> O pipeline completo de lançamento — incluindo o instalador do Windows e as compilações do Cellar — está em [`.github/workflows/test_build.yml`](../.github/workflows/test_build.yml), a referência oficial de como os artefatos oficiais são produzidos.

---

## 1. Obtendo o código-fonte

```bash
git clone https://github.com/Tenshou170/Hachimi-Edge.git
cd Hachimi-Edge
```

Se a árvore de trabalho do git tiver alterações não commitadas, a compilação do Zygisk adiciona o sufixo `-dirty` à versão do módulo. Defina `HACHIMI_IGNORE_DIRTY=true` para suprimir isso (o CI fixa como `false` para que modificações locais fiquem visíveis nos artefatos).

---

## 2. Pré-requisitos

### Toolchain Rust
Instale a versão estável mais recente do Rust via [rustup.rs](https://rustup.rs/).

### Windows (x64 MSVC)
- **No Windows:** toolchain MSVC padrão, instalada pelo Visual Studio ou pelas Build Tools independentes.
- **No Linux / macOS:** `cargo-xwin`, que baixa e configura o sysroot do MSVC automaticamente:
  ```bash
  cargo install cargo-xwin
  ```
  A toolchain LLVM/Clang/LLD também é necessária para a etapa de linkedição; garanta que o `llvm-lib` esteja disponível (no CI é um symlink para o `llvm-ar`):
  ```bash
  # Debian / Ubuntu (LLVM 19, como no CI)
  sudo apt-get install -y clang-19 lld-19 llvm-19
  sudo ln -sf /usr/lib/llvm-19/bin/llvm-ar /usr/local/bin/llvm-lib

  # macOS
  brew install llvm
  sudo ln -sf "$(brew --prefix llvm)/bin/llvm-ar" /usr/local/bin/llvm-lib
  ```

### Android (ARM64)
- Recomenda-se o **Android NDK r27d LTS**.
- Adicione o destino ARM64 do Rust:
  ```bash
  rustup target add aarch64-linux-android
  ```
- O `zip` é necessário no host ao empacotar o módulo Zygisk.

---

## 3. Configuração do NDK

O sistema de compilação descobre o caminho do NDK automaticamente, nesta ordem de prioridade:

1. Variável de ambiente `$ANDROID_NDK_ROOT`
2. Variável de ambiente `$ANDROID_NDK_HOME`
3. Um symlink ou diretório `./ndk` na raiz do projeto

**Opção A — Variável de ambiente (recomendado para CI e compilações pontuais):**
```bash
export ANDROID_NDK_ROOT=/caminho/para/android-ndk-r27d
```

**Opção B — Symlink local (recomendado para o desenvolvimento do dia a dia):**
```bash
# Linux / macOS
ln -s /caminho/para/android-ndk-r27d ndk

# Windows (Prompt de comando)
mklink /J ndk C:\caminho\para\android-ndk-r27d
```

Não é necessário editar o `.cargo/config.toml` manualmente — o sistema de compilação lida com todos os hosts automaticamente.

---

## 4. Compilação

### Windows (x64 MSVC)

**Verificação do compilador:**
```bash
cargo xcheck
```

**Compilação de depuração:**
```bash
./tools/windows/build.sh
```

**Compilação de lançamento (release):**
```bash
RELEASE=1 ./tools/windows/build.sh
```

O script detecta o sistema operacional do host automaticamente:
- No Windows: compila nativamente com a toolchain MSVC.
- No Linux / macOS: compilação cruzada via `cargo-xwin`.

**Saída:** `build/hachimi.dll`, além de `build/blake3.json` (gerado quando o `b3sum` está instalado).

### Android (ARM64)

**Verificação do compilador:**
```bash
cargo acheck
```

**Compilação de depuração:**
```bash
./tools/android/build.sh
```

**Compilação de lançamento (release):**
```bash
RELEASE=1 ./tools/android/build.sh
```

Compila com o **nível de API 24** e **alinhamento de páginas de 16 KB**, garantindo compatibilidade total do Android 7.0 ao Android 15+.

**Saída:** `build/libmain-arm64-v8a.so`, além de `build/sha256.json` (gerado quando `sha256sum` ou `shasum` está disponível).

### Módulo Zygisk (Android)

```bash
RELEASE=1 ./tools/android/build_zygisk.sh
```

Executa a compilação de lançamento do Android, remove símbolos de depuração com o `llvm-strip` do NDK, empacota o módulo a partir de `tools/android/zygisk-template` e grava checksums SHA-256 por arquivo.

**Saída:** `build/zygisk-hachimi-edge-v<versão>-<commit>[-dirty]-release.zip` (módulo instalável no Magisk/KernelSU). Requer `zip` no host.

### Argumentos extras do Cargo

Ambos os scripts de compilação repassam argumentos adicionais ao cargo via `CARGOARGS`:

```bash
RELEASE=1 CARGOARGS="--features some_feature" ./tools/windows/build.sh
```

---

## 5. Referência de aliases do Cargo

Definidos em `.cargo/config.toml`. Os aliases de `check`, `clippy` e `test` espelham os comandos do CI byte a byte, incluindo `-D warnings`, então uma execução local reproduz o CI exatamente. Para produzir artefatos de lançamento, use os scripts de compilação acima em vez dos aliases de build.

| Alias | Descrição |
|---|---|
| `cargo xcheck` | Verificação do compilador para o destino Windows MSVC (perfil release) |
| `cargo xbuild` | Compilação para Windows MSVC via `cargo-xwin` (perfil release) |
| `cargo xclippy` | Lint Clippy para o destino Windows MSVC (`-D warnings`) |
| `cargo xtest` | Compilação dos testes para o destino Windows MSVC (`--no-run`) |
| `cargo acheck` | Verificação do compilador para o destino Android ARM64 (perfil release) |
| `cargo abuild` | Compilação para Android ARM64 (perfil release) |
| `cargo aclippy` | Lint Clippy para o destino Android ARM64 (`-D warnings`) |
| `cargo atest` | Compilação dos testes para o destino Android ARM64 (`--no-run`) |
