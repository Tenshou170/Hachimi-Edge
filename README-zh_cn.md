<div align="center">
  <img src="assets/icon.png" width="128" height="128" alt="Hachimi Edge Logo">
  <h1>Hachimi Edge</h1>
  <p><b>UM:PD 游戏增强与翻译插件</b></p>

  <p><a href="README.md">English</a> | 简体中文 | <a href="README-zh_tw.md">繁體中文</a></p>

  <p>
    <a href="https://github.com/Tenshou170/Hachimi-Edge/actions"><img src="https://img.shields.io/github/actions/workflow/status/Tenshou170/Hachimi-Edge/test_build.yml?branch=main&label=Build&style=for-the-badge" alt="Build Status"></a><img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Android-blue?style=for-the-badge" alt="Target Platforms">
  </p>
  <p>
    <a href="https://discord.gg/YjBgmuqqYr"><img src="https://dcbadge.limes.pink/api/server/https://discord.gg/YjBgmuqqYr" alt="Discord Server"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL%203.0-blue.svg?style=flat-square" alt="License"></a>
  </p>
</div>

<div align="center">
  <img height="400" src="assets/Screenshot.png">
</div>

## 分享与传播指南

本项目通过修改游戏运行时行为实现功能，违反了目标游戏的服务条款（TOS）。为降低项目及用户群体的风险，请遵守以下原则：

- **请勿在公开网站、论坛或社交媒体平台**直接发布本仓库、项目网站或相关工具的链接。
- 仅限通过私信或自建私密群组分享相关信息。
- 在公开场合提及目标游戏时，请使用代称（如“UM:PD”或“某赛马拟人化游戏”），避免被搜索引擎索引。

## 功能特性

- **高质量本地化支持：** 内置文本格式化系统（支持复数形式、序数词及动态布局适配），无需手动修改游戏资源文件。
  - 支持组件：
    - 界面文本
    - 数据库条目（`master.mdb`、技能名称与描述）
    - 赛事剧情与主线/育成对话
    - 歌曲歌词
    - 动态纹理与图集替换
  - 可配置语言系统，支持自定义本地化字典。
- **内置控制面板：** 内置 GUI 配置编辑器，支持在游戏运行时即时调整设置，无需重启游戏。
- **自动更新机制：** 内置更新器可在游戏运行期间后台下载并实时加载最新翻译包。
- **画质与性能优化：** 提供解锁帧率限制（FPS Unlock）及分辨率缩放等图形配置项。
- **跨平台支持：** 原生支持 Windows（DirectX 11 代理 DLL）与 Android（Zygisk / Dobby 内联 Hook）。

## 安装指南

请参阅官方[入门指南文档](https://hachimi.noccu.art/zh-cn/docs/hachimi/getting-started.html)。

## 从源码构建

如需从源码编译构建 Hachimi Edge，请参阅 [BUILDING-zh_cn.md](BUILDING-zh_cn.md)。

## 致谢与参考

Hachimi Edge 的开发借鉴了以下开源项目的架构设计与技术实现：

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

## 许可证

本项目基于 [GNU General Public License v3.0](LICENSE) 开源许可证发布。
