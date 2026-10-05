<div align="center">
  <img src="../assets/icon.png" width="128" height="128" alt="Hachimi Edge Logo">
  <h1>Hachimi Edge</h1>
  <p><b>UM:PD 遊戲強化與翻譯模組</b></p>

  <p><a href="../README.md">English</a> | <a href="README-es.md">Español</a> | <a href="README-fil.md">Filipino</a> | <a href="README-id.md">Bahasa Indonesia</a> | <a href="README-ja.md">日本語</a> | <a href="README-ko.md">한국어</a> | <a href="README-pt-br.md">Português (Brasil)</a> | <a href="README-ru.md">Русский</a> | <a href="README-vi.md">Tiếng Việt</a> | <a href="README-zh_cn.md">简体中文</a> | 繁體中文</p>

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

## ⚠️ 分享與散佈指南

本專案透過修改遊戲運行時行為實現功能，違反了目標遊戲的服務條款（TOS）。如果遊戲開發商得知本專案的存在，幾乎必然會設法將其清除。為降低專案及使用者群體的風險，請遵守以下原則：

- **請勿在公開網站、論壇或社交媒體平台**直接發布本倉庫、專案網站或相關工具的連結。
- 僅限透過私訊或自建私密群組分享相關資訊。
- 在公開場合提及目標遊戲時，請使用代稱（如「UM:PD」或「那個賽馬遊戲」），避免被搜尋引擎索引。

**否則就隨意分享吧，讓幾十位 Hachimi 使用者一起遭殃。悉聽尊便。**

> [!WARNING]
> **如果你執意要分享**
> 那也請自便，但我們懇請你在公開場合盡量使用「UM:PD」或「那個賽馬遊戲」等代稱來指代該遊戲，而非遊戲的真實名稱，以免被搜尋引擎收錄。

## 功能特色

- **高品質在地化支援：** 內建文字格式化處理系統（支援複數型、序數詞及動態版面配置），無需手動修改遊戲資源檔。支援翻譯遊戲內絕大多數元件，無需手動修改資源檔！
  - 支援元件包括：
    - UI 介面文字
    - 資料庫條目（`master.mdb`、技能名稱與描述）
    - 賽事劇情與主線／培育對話
    - 歌曲歌詞
    - 動態材質與圖集替換
  - 可設定語言系統，支援自訂在地化字典。
- **自動機器翻譯（可選）：** 為在地化語言包尚未涵蓋的文字提供社群來源及機器翻譯，直接在遊戲內套用。
- **內建控制面板：** 內建 GUI 設定編輯器，支援在遊戲執行期間即時調整設定，無需重啟遊戲。
- **自動更新機制：** 內建更新器可在遊戲執行期間於背景下載並即時載入最新翻譯包。
- **畫質與效能最佳化：** 提供解鎖幀率限制（FPS Unlock）及解析度縮放等圖形設定選項。
- **跨平台支援：** 原生支援 Windows（DirectX 11 代理 DLL）與 Android（Zygisk / Dobby 內聯 Hook）。
- **Windows 桌面整合：** Discord Rich Presence、Windows 媒體傳輸控制（SMTC，含即時歌曲封面縮圖）以及工作列進度顯示。

## 安裝指南

請參閱官方[快速開始指南文件](https://hachimi.noccu.art/zh-tw/docs/hachimi/getting-started.html)。

## 從原始碼構建

如需從原始碼編譯構建 Hachimi Edge，請參閱 [BUILDING-zh_tw.md](BUILDING-zh_tw.md)。

## AI / LLM 使用聲明

Hachimi Edge 的部分內容——包括程式碼、文件與翻譯文本——曾借助大型語言模型（LLM）及其他 AI 工具撰寫或潤飾。所有 AI 輔助產出在收錄前均經過人工審校，但仍可能存在錯誤、誤譯或非預期的行為。請自行斟酌使用；我們不對 AI 生成內容的正確性作任何保證。

## 致謝與參考

Hachimi Edge 的開發借鏡了以下開源專案的架構設計與技術實現：

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

## 授權條款

本專案基於 [GNU General Public License v3.0](../LICENSE) 開源授權條款釋出。
