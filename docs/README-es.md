<div align="center">
  <img src="../assets/icon.png" width="128" height="128" alt="Hachimi Edge Logo">
  <h1>Hachimi Edge</h1>
  <p><b>Mod de mejora y traducción para UM:PD</b></p>

  <p><a href="../README.md">English</a> | Español | <a href="README-fil.md">Filipino</a> | <a href="README-id.md">Bahasa Indonesia</a> | <a href="README-ja.md">日本語</a> | <a href="README-ko.md">한국어</a> | <a href="README-pt-br.md">Português (Brasil)</a> | <a href="README-ru.md">Русский</a> | <a href="README-vi.md">Tiếng Việt</a> | <a href="README-zh_cn.md">简体中文</a> | <a href="README-zh_tw.md">繁體中文</a></p>

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

## ⚠️ Guías de compartición y redistribución

Este proyecto modifica el comportamiento del juego en tiempo de ejecución y viola los Términos de Servicio (TOS) de la aplicación objetivo. El desarrollador del juego casi con certeza querrá eliminarlo en cuanto se entere de su existencia. Para minimizar el riesgo para el proyecto y su base de usuarios, observa las siguientes pautas:

- **No publiques enlaces directos** a este repositorio, al sitio web del proyecto ni a las herramientas asociadas en sitios web, foros o redes sociales públicos.
- Comparte la información exclusivamente por mensaje privado o en grupos comunitarios autogestionados.
- Al mencionar la aplicación objetivo en público, usa referencias indirectas (como «UM:PD» o «el juego de los caballos») para evitar el indexado por motores de búsqueda.

**O compártelos y arruínaselo a las decenas de usuarios de Hachimi. Tú decides.**

> [!WARNING]
> **Si de todas formas vas a compartirlo**
> Haz lo que tengas que hacer, pero te pediríamos respetuosamente que intentes referirte al juego como «UM:PD» o «el juego de los caballos» en lugar de su nombre real, para evitar el análisis de los motores de búsqueda.

## Características

- **Localizaciones de alta calidad:** Soporte de formato de texto avanzado (formas de plural, números ordinales, ajuste dinámico del diseño) sin modificaciones manuales de recursos. ¡También traduce la mayoría de los componentes del juego; sin parcheo manual de recursos!
  - Componentes compatibles:
    - Texto de la interfaz
    - Entradas de la base de datos (`master.mdb`, nombres y descripciones de habilidades)
    - Historias de carreras y diálogos del escenario principal
    - Letras de canciones
    - Reemplazo dinámico de texturas y atlas de sprites
  - Sistema de idiomas configurable con soporte para diccionarios de localización personalizados.
- **Traducción Automática (opcional):** Traducciones de la comunidad y automáticas para textos aún no cubiertos por los paquetes de localización, aplicadas directamente en el juego.
- **Configuración en el juego:** El editor de configuración con GUI integrado permite ajustar las opciones en tiempo real sin reiniciar la aplicación.
- **Actualizaciones automáticas de localización:** El actualizador integrado descarga y recarga los paquetes de traducción actualizados directamente durante la ejecución del juego.
- **Mejoras gráficas:** Funciones de optimización del dispositivo, incluido el desbloqueo de la tasa de fotogramas (FPS unlock) y el escalado de resolución.
- **Multiplataforma:** Soporte nativo para Windows (DLL proxy de DirectX 11) y Android (hooks en línea Zygisk / Dobby).
- **Integraciones de escritorio de Windows:** Discord Rich Presence, controles multimedia de Windows Media Transport (SMTC) con miniaturas de portadas en directo e informe de progreso en la barra de tareas.

## Instalación

Consulta la [documentación oficial de introducción](https://hachimi.noccu.art/docs/hachimi/getting-started.html).

## Compilación desde el código fuente

Las instrucciones detalladas de compilación y configuración del entorno están documentadas en [BUILDING-es.md](BUILDING-es.md).

## Aviso de uso de IA / LLM

Partes de Hachimi Edge —incluido el código, la documentación y el contenido de las traducciones— se han escrito o refinado con la ayuda de modelos de lenguaje de gran escala (LLM) y otras herramientas de IA. Todo el contenido asistido por IA se revisa antes de incluirse, pero aún puede contener imprecisiones, errores de traducción o comportamientos no previstos. Úsalo bajo tu propio criterio; no se ofrece ninguna garantía de corrección para el contenido generado por IA.

## Créditos y referencias

Hachimi Edge incorpora conceptos y técnicas establecidos por los siguientes proyectos de código abierto:

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

## Licencia

Este proyecto está licenciado bajo la [Licencia Pública General de GNU v3.0](../LICENSE).
