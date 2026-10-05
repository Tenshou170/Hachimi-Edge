<div align="center">
  <img src="../assets/icon.png" width="128" height="128" alt="Hachimi Edge Logo">
  <h1>Hachimi Edge</h1>
  <p><b>Mod de aprimoramento e tradução para UM:PD</b></p>

  <p><a href="../README.md">English</a> | <a href="README-es.md">Español</a> | <a href="README-fil.md">Filipino</a> | <a href="README-id.md">Bahasa Indonesia</a> | <a href="README-ja.md">日本語</a> | <a href="README-ko.md">한국어</a> | Português (Brasil) | <a href="README-ru.md">Русский</a> | <a href="README-vi.md">Tiếng Việt</a> | <a href="README-zh_cn.md">简体中文</a> | <a href="README-zh_tw.md">繁體中文</a></p>

  <p>
    <a href="https://github.com/Tenshou170/Hachimi-Edge/actions"><img src="https://img.shields.io/github/actions/workflow/status/Tenshou170/Hachimi-Edge/test_build.yml?branch=main&label=Build&style=for-the-badge" alt="Build Status"></a> <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Android-blue?style=for-the-badge" alt="Target Platforms">
  </p>
  <p>
    <a href="https://discord.gg/YjBgmuqqYr"><img src="https://dcbadge.limes.pink/api/server/https://discord.gg/YjBgmuqqYr" alt="Discord Server"></a>
    <a href="../LICENSE"><img src="https://img.shields.io/badge/License-GPL%203.0-blue.svg?style=flat-square" alt="License"></a>
  </p>
</div>

<div align="center">
  <img height="400" src="../assets/Screenshot.png">
</div>

## ⚠️ Diretrizes de Compartilhamento e Redistribuição

Este projeto modifica o comportamento do jogo em tempo de execução e viola os Termos de Serviço (TOS) do aplicativo alvo. É bem possível que o desenvolvedor do jogo queira vê-lo eliminado assim que descobri-lo. Para minimizar o risco ao projeto e à sua base de usuários, siga as diretrizes abaixo:

- **Não publique links diretos** para este repositório, para o site do projeto ou para as ferramentas associadas em sites, fóruns ou redes sociais públicos.
- Compartilhe informações exclusivamente por mensagem privada ou em grupos comunitários autogerenciados.
- Ao mencionar o aplicativo alvo publicamente, use referências indiretas (como "UM:PD" ou "O Jogo dos Cavalinhos") para evitar indexação por mecanismos de busca.

**Ou compartilhe e estrague tudo para as dezenas de usuários do Hachimi. Você que sabe.**

> [!WARNING]
> **Se você for compartilhar mesmo assim**
> Faça o que precisa, mas pediríamos respeitosamente que tentasse se referir ao jogo como "UM:PD" ou "O Jogo dos Cavalinhos" em vez do nome real, para evitar a indexação dos mecanismos de busca.

## Funcionalidades

- **Localizações de alta qualidade:** Suporte avançado a formatação de texto (plurais, números ordinais, ajuste dinâmico de layout) sem modificações manuais de assets. Também traduz a maioria dos componentes do jogo; sem necessidade de patch manual de assets!
  - Componentes suportados:
    - Texto da interface
    - Entradas do banco de dados (`master.mdb`, nomes e descrições de habilidades)
    - Histórias de corrida e diálogos do cenário principal
    - Letras de músicas
    - Substituição dinâmica de texturas e atlas de sprites
  - Sistema de idiomas configurável com suporte a dicionários de localização personalizados.
- **Tradução Automática (opcional):** Traduções da comunidade e automáticas para textos ainda não cobertos pelos pacotes de localização, aplicadas diretamente no jogo.
- **Configuração no jogo:** O editor de configurações com GUI integrado permite ajustes em tempo real sem reiniciar o aplicativo.
- **Atualizações automáticas de localização:** O atualizador integrado baixa e recarrega pacotes de tradução atualizados diretamente durante a execução do jogo.
- **Aprimoramentos gráficos:** Recursos de otimização do dispositivo, incluindo desbloqueio de taxa de quadros (FPS unlock) e escalonamento de resolução.
- **Multiplataforma:** Suporte nativo para Windows (DLL proxy DirectX 11) e Android (hooks inline Zygisk / Dobby).
- **Integrações com a área de trabalho do Windows:** Discord Rich Presence, controles de mídia do Windows Media Transport (SMTC) com miniaturas de capas ao vivo e relatório de progresso na barra de tarefas.

## Instalação

Consulte a [documentação oficial de primeiros passos](https://hachimi.noccu.art/docs/hachimi/getting-started.html).

## Compilando a partir do código-fonte

As instruções detalhadas de compilação e configuração do ambiente estão em [BUILDING-pt-br.md](BUILDING-pt-br.md).

## Aviso de uso de IA / LLM

Partes do Hachimi Edge — incluindo código, documentação e conteúdo de traduções — foram escritas ou refinadas com a ajuda de grandes modelos de linguagem (LLMs) e outras ferramentas de IA. Tudo o que conta com auxílio de IA é revisado antes de ser incluído, mas ainda pode conter imprecisões, erros de tradução ou comportamentos não intencionais. Use por sua conta e risco; nenhuma garantia de correção é fornecida para conteúdo gerado por IA.

## Créditos e referências

O Hachimi Edge incorpora conceitos e técnicas estabelecidos pelos seguintes projetos de código aberto:

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

## Licença

Este projeto está licenciado sob a [GNU General Public License v3.0](../LICENSE).
