<div align="center">
  <img src="../assets/icon.png" width="128" height="128" alt="Hachimi Edge Logo">
  <h1>Hachimi Edge</h1>
  <p><b>UM:PD 向けのゲーム強化・翻訳Mod</b></p>

  <p><a href="../README.md">English</a> | <a href="README-es.md">Español</a> | <a href="README-fil.md">Filipino</a> | <a href="README-id.md">Bahasa Indonesia</a> | 日本語 | <a href="README-ko.md">한국어</a> | <a href="README-pt-br.md">Português (Brasil)</a> | <a href="README-ru.md">Русский</a> | <a href="README-vi.md">Tiếng Việt</a> | <a href="README-zh_cn.md">简体中文</a> | <a href="README-zh_tw.md">繁體中文</a></p>

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

## ⚠️ 共有・再配布ガイドライン

本プロジェクトはゲームの実行時動作を改変するものであり、対象アプリケーションの利用規約（TOS）に違反します。開発元に存在が知れた場合、ほぼ間違いなく排除を望まれるでしょう。プロジェクトとユーザーコミュニティへのリスクを最小限に抑えるため、以下のガイドラインを守ってください：

- 本リポジトリ、プロジェクトのウェブサイト、関連ツールへの**直接リンクを公開サイト、フォーラム、SNS に投稿しないでください**。
- 情報の共有は、個人的なダイレクトメッセージまたは自己管理のコミュニティグループに限定してください。
- 公の場で対象ゲームに言及する際は、検索エンジンにインデックスされないよう、「UM:PD」や「あの競馬擬人化ゲーム」などの間接的な表現を使用してください。

**それでも共有して、多数の Hachimi ユーザーに迷惑をかけるかどうかは、あなた次第です。**

> [!WARNING]
> **どうしても共有する場合は**
> ご自由になさってください。ただし、検索エンジンの解析を避けるため、ゲームの実際の名称ではなく「UM:PD」や「あの競馬擬人化ゲーム」といった呼称を使うよう、お願いいたします。

## 機能

- **高品質なローカライズ:** 複数形・序数・動的レイアウト調整などの高度なテキスト整形に対応し、アセットの手動改変は不要です。ゲーム内のほとんどのコンポーネントの翻訳にも対応。アセットの手動パッチ適用は必要ありません！
  - 対応コンポーネント：
    - UI テキスト
    - データベースエントリ（`master.mdb`、スキル名・説明）
    - レースストーリーとメインシナリオのダイアログ
    - 楽曲の歌詞
    - テクスチャおよびスプライトアトラスの動的置換
  - カスタムローカライズ辞書に対応した、設定可能な言語システム。
- **自動機械翻訳（任意）：** ローカライズパッケージで未対応のテキストに対し、コミュニティ提供および機械翻訳をゲーム内で直接適用します。
- **ゲーム内設定：** 組み込みの GUI 設定エディタにより、アプリを再起動せずにリアルタイムで設定を調整できます。
- **ローカライズ自動更新：** 内蔵アップデータが、最新の翻訳パッケージをゲームの実行中にダウンロードし、そのままゲーム内でリロードします。
- **グラフィック強化：** フレームレート制限の解除（FPS アンロック）や解像度スケーリングなど、デバイスの性能を引き出す機能を搭載。
- **クロスプラットフォーム：** Windows（DirectX 11 プロキシ DLL）と Android（Zygisk / Dobby インラインフック）にネイティブ対応。
- **Windows デスクトップ統合：** Discord Rich Presence、Windows メディアトランスポート（SMTC）によるジャケットサムネイル付きメディア操作、タスクバーへの進行状況表示。

## インストール

公式の[はじめにドキュメント](https://hachimi.noccu.art/docs/hachimi/getting-started.html)を参照してください。

## ソースからのビルド

詳細なビルド手順と環境構築については [BUILDING-ja.md](BUILDING-ja.md) を参照してください。

## AI / LLM 利用に関する声明

Hachimi Edge の一部（コード、ドキュメント、翻訳コンテンツを含む）は、大規模言語モデル（LLM）やその他の AI ツールの支援を受けて作成・推敲されています。AI 支援の出力はすべて取り込み前にレビューされていますが、誤り、誤訳、意図しない動作が含まれる可能性が残っています。ご自身の判断でご利用ください。AI 生成コンテンツの正確性について一切保証いたしません。

## クレジット・参考文献

Hachimi Edge は、以下のオープンソースプロジェクトで確立された概念や技術を取り入れています：

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

## ライセンス

本プロジェクトは [GNU General Public License v3.0](../LICENSE) の下でライセンスされています。
