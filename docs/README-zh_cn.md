<div align="center">
  <img src="../assets/icon.png" width="128" height="128" alt="Hachimi Edge Logo">
  <h1>Hachimi Edge</h1>
  <p><b>UM:PD 游戏增强与翻译插件</b></p>

  <p><a href="../README.md">English</a> | <a href="README-es.md">Español</a> | <a href="README-fil.md">Filipino</a> | <a href="README-id.md">Bahasa Indonesia</a> | <a href="README-ja.md">日本語</a> | <a href="README-ko.md">한국어</a> | <a href="README-pt-br.md">Português (Brasil)</a> | <a href="README-ru.md">Русский</a> | <a href="README-vi.md">Tiếng Việt</a> | 简体中文 | <a href="README-zh_tw.md">繁體中文</a></p>

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

## ⚠️ 分享与传播指南

本项目通过修改游戏运行时行为实现功能，违反了目标游戏的服务条款（TOS）。如果游戏开发商得知本项目的存在，几乎必然会设法将其清除。为降低项目及用户群体的风险，请遵守以下原则：

- **请勿在公开网站、论坛或社交媒体平台**直接发布本仓库、项目网站或相关工具的链接。
- 仅限通过私信或自建私密群组分享相关信息。
- 在公开场合提及目标游戏时，请使用代称（如"UM:PD"或"某赛马拟人化游戏"），避免被搜索引擎索引。

**否则随意分享吧，让几十位 Hachimi 用户一起遭殃。悉听尊便。**

> [!WARNING]
> **如果你执意要分享**
> 那也请随意，但我们恳请你在公开场合尽量使用"UM:PD"或"某赛马拟人化游戏"等代称来指代该游戏，而非游戏的真实名称，以免被搜索引擎收录。

## 功能特性

- **高质量本地化支持：** 内置文本格式化系统（支持复数形式、序数词及动态布局适配），无需手动修改游戏资源文件。支持翻译游戏内绝大多数组件，无需手动修改资源文件！
  - 支持组件：
    - 界面文本
    - 数据库条目（`master.mdb`、技能名称与描述）
    - 赛事剧情与主线/育成对话
    - 歌曲歌词
    - 动态纹理与图集替换
  - 可配置语言系统，支持自定义本地化字典。
- **自动机器翻译（可选）：** 为本地化语言包尚未覆盖的文本提供社区来源及机器翻译，直接在游戏内应用。
- **内置控制面板：** 内置 GUI 配置编辑器，支持在游戏运行时即时调整设置，无需重启游戏。
- **自动更新机制：** 内置更新器可在游戏运行期间后台下载并实时加载最新翻译包。
- **画质与性能优化：** 提供解锁帧率限制（FPS Unlock）及分辨率缩放等图形配置项。
- **跨平台支持：** 原生支持 Windows（DirectX 11 代理 DLL）与 Android（Zygisk / Dobby 内联 Hook）。
- **Windows 桌面集成：** Discord Rich Presence、Windows 媒体传输控制（SMTC，含实时歌曲封面缩略图）以及任务栏进度显示。

## 安装指南

请参阅官方[入门指南文档](https://hachimi.noccu.art/zh-cn/docs/hachimi/getting-started.html)。

## 从源码构建

如需从源码编译构建 Hachimi Edge，请参阅 [BUILDING-zh_cn.md](BUILDING-zh_cn.md)。

## AI / LLM 使用声明

Hachimi Edge 的部分内容——包括代码、文档与翻译文本——曾借助大语言模型（LLM）及其他 AI 工具撰写或润色。所有 AI 辅助产出在收录前均经过人工审校，但仍可能存在错误、误译或非预期的行为。请自行斟酌使用；我们不对 AI 生成内容的正确性作任何保证。

## 致谢与参考

Hachimi Edge 的开发借鉴了以下开源项目的架构设计与技术实现：

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

## 许可证

本项目基于 [GNU General Public License v3.0](../LICENSE) 开源许可证发布。
