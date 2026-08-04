<div align="center">
  <img src="assets/icon.png" width="128" height="128" alt="Hachimi Edge Logo">
  <h1>Hachimi Edge</h1>
  <p><b>UM:PD 遊戲強化與翻譯模組</b></p>

  <p><a href="README.md">English</a> | <a href="README-zh_cn.md">簡體中文</a> | 繁體中文</p>

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

## 分享與散佈指南

本專案透過修改遊戲運行時行為實現功能，違反了目標遊戲的服務條款（TOS）。為降低專案及使用者群體的風險，請遵守以下原則：

- **請勿在公開網站、論壇或社交媒體平台**直接發布本倉庫、專案網站或相關工具的連結。
- 僅限透過私訊或自建私密群組分享相關資訊。
- 在公開場合提及目標遊戲時，請使用代稱（如「UM:PD」或「那個賽馬遊戲」），避免被搜尋引擎索引。

## 功能特色

- **高品質在地化支援：** 內建文字格式化處理系統（支援複數型、序數詞及動態版面配置），無需手動修改遊戲資源檔。
  - 支援元件包括：
    - UI 介面文字
    - 資料庫條目（`master.mdb`、技能名稱與描述）
    - 賽事劇情與主線／培育對話
    - 歌曲歌詞
    - 動態材質與圖集替換
  - 可設定語言系統，支援自訂在地化字典。
- **內建控制面板：** 內建 GUI 設定編輯器，支援在遊戲執行期間即時調整設定，無需重啟遊戲。
- **自動更新機制：** 內建更新器可在遊戲執行期間於背景下載並即時載入最新翻譯包。
- **畫質與效能最佳化：** 提供解鎖幀率限制（FPS Unlock）及解析度縮放等圖形設定選項。
- **跨平台支援：** 原生支援 Windows（DirectX 11 代理 DLL）與 Android（Zygisk / Dobby 內聯 Hook）。

## 安裝指南

請參閱官方[快速開始指南文件](https://hachimi.noccu.art/zh-tw/docs/hachimi/getting-started.html)。

## 從原始碼構建

如需從原始碼編譯構建 Hachimi Edge，請參閱 [BUILDING-zh_tw.md](BUILDING-zh_tw.md)。

## 致謝與參考

Hachimi Edge 的開發借鏡了以下開源專案的架構設計與技術實現：

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

## 授權條款

本專案基於 [GNU General Public License v3.0](LICENSE) 開源授權條款釋出。