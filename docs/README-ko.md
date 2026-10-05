<div align="center">
  <img src="../assets/icon.png" width="128" height="128" alt="Hachimi Edge Logo">
  <h1>Hachimi Edge</h1>
  <p><b>UM:PD 게임 개선 및 번역 모드</b></p>

  <p><a href="../README.md">English</a> | <a href="README-es.md">Español</a> | <a href="README-fil.md">Filipino</a> | <a href="README-id.md">Bahasa Indonesia</a> | <a href="README-ja.md">日本語</a> | 한국어 | <a href="README-pt-br.md">Português (Brasil)</a> | <a href="README-ru.md">Русский</a> | <a href="README-vi.md">Tiếng Việt</a> | <a href="README-zh_cn.md">简体中文</a> | <a href="README-zh_tw.md">繁體中文</a></p>

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

## ⚠️ 공유 및 재배포 가이드라인

이 프로젝트는 게임의 런타임 동작을 수정하며 대상 애플리케이션의 서비스 약관(TOS)을 위반합니다. 게임 개발사는 이 프로젝트의 존재를 알게 되면 확실히 없애려 할 것입니다. 프로젝트와 사용자 커뮤니티에 대한 위험을 최소화하기 위해 다음 가이드라인을 지켜 주세요:

- 이 저장소, 프로젝트 웹사이트 또는 관련 도구로 가는 **직접 링크를 공개 웹사이트, 포럼, 소셜 미디어에 게시하지 마세요.**
- 정보 공유는 개인 메시지 또는 자체 운영 커뮤니티 그룹으로만 진행해 주세요.
- 공개적인 자리에서 대상 게임을 언급할 때는 검색 엔진 색인을 피하기 위해 "UM:PD" 또는 "그 경마 게임" 같은 간접 표현을 사용해 주세요.

**아니면 그냥 공유해서 수십 명의 Hachimi 사용자에게 피해를 주셔도 됩니다. 본인 선택입니다.**

> [!WARNING]
> **어차피 공유하실 거라면**
> 하고 싶으신 대로 하시되, 검색 엔진 파싱을 피하기 위해 게임의 실제 이름 대신 "UM:PD"나 "그 경마 게임"으로 표기해 주시면 감사하겠습니다.

## 기능

- **고품질 현지화:** 복수형, 서수, 동적 레이아웃 맞춤 등 고급 텍스트 서식을 지원하며 수동 에셋 수정이 필요 없습니다. 게임 내 대부분의 구성 요소 번역도 지원하며, 수동 에셋 패치가 필요하지 않습니다!
  - 지원 구성 요소:
    - UI 텍스트
    - 데이터베이스 항목(`master.mdb`, 스킬 이름 및 설명)
    - 레이스 스토리 및 메인 시나리오 대화
    - 가사
    - 동적 텍스처 및 스프라이트 아틀라스 교체
  - 사용자 지정 현지화 사전을 지원하는 구성 가능한 언어 시스템.
- **자동 기계 번역(선택):** 현지화 패키지가 아직 다루지 않는 텍스트에 대해 커뮤니티 제공 번역과 기계 번역을 게임 내에 직접 적용합니다.
- **게임 내 설정:** 내장 GUI 설정 편집기로 애플리케이션을 다시 시작하지 않고도 실시간으로 설정을 조정할 수 있습니다.
- **현지화 자동 업데이트:** 내장 업데이터가 실행 중인 게임 내에서 직접 최신 번역 패키지를 다운로드하고 다시 불러옵니다.
- **그래픽 향상:** 프레임 레이트 제한 해제(FPS 언락) 및 해상도 스케일링 등 기기 성능을 최대한 활용하는 기능.
- **크로스 플랫폼:** Windows(DirectX 11 프록시 DLL)와 Android(Zygisk / Dobby 인라인 훅) 네이티브 지원.
- **Windows 데스크톱 통합:** Discord Rich Presence, 라이브 재킷 썸네일이 포함된 Windows 미디어 전송(SMTC) 미디어 컨트롤, 작업 표시줄 진행률 표시.

## 설치

공식 [시작하기 문서](https://hachimi.noccu.art/docs/hachimi/getting-started.html)를 참조하세요.

## 소스에서 빌드

상세한 컴파일 및 환경 설정 방법은 [BUILDING-ko.md](BUILDING-ko.md)에 문서화되어 있습니다.

## AI / LLM 사용 고지

Hachimi Edge의 일부(코드, 문서, 번역 콘텐츠 포함)는 대규모 언어 모델(LLM) 및 기타 AI 도구의 도움을 받아 작성되거나 다듬어졌습니다. AI 지원 출력은 포함되기 전에 검토되지만 여전히 부정확성, 오역 또는 의도하지 않은 동작이 포함될 수 있습니다. 본인 판단에 따라 사용하시기 바랍니다. AI 생성 콘텐츠의 정확성에 대해 어떠한 보증도 제공하지 않습니다.

## 크레딧 및 참고 자료

Hachimi Edge는 다음 오픈 소스 프로젝트에서 확립한 개념과 기술을 활용합니다:

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

## 라이선스

이 프로젝트는 [GNU General Public License v3.0](../LICENSE)에 따라 라이선스가 부여됩니다.
