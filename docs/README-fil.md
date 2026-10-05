<div align="center">
  <img src="../assets/icon.png" width="128" height="128" alt="Hachimi Edge Logo">
  <h1>Hachimi Edge</h1>
  <p><b>Mod na pagpapahusay at pagsasalin ng laro para sa UM:PD</b></p>

  <p><a href="../README.md">English</a> | <a href="README-es.md">Español</a> | Filipino | <a href="README-id.md">Bahasa Indonesia</a> | <a href="README-ja.md">日本語</a> | <a href="README-ko.md">한국어</a> | <a href="README-pt-br.md">Português (Brasil)</a> | <a href="README-ru.md">Русский</a> | <a href="README-vi.md">Tiếng Việt</a> | <a href="README-zh_cn.md">简体中文</a> | <a href="README-zh_tw.md">繁體中文</a></p>

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

## ⚠️ Mga Alituntunin sa Pagbabahagi at Muling Pagpapakalat

Binabago ng proyektong ito ang pag-uugali ng laro sa runtime at lumalabag sa Mga Tuntunin ng Serbisyo (TOS) ng target na aplikasyon. Halos tiyak na gustong alisin ng developer ng laro ang proyektong ito kapag nalaman nila ito. Para mabawasan ang panganib sa proyekto at sa mga gumagamit nito, sundin ang mga alituntuning ito:

- **Huwag mag-post ng mga direktang link** sa repositoryong ito, sa website ng proyekto, o sa mga kaugnay na tool sa mga pampublikong website, forum, o social media.
- Ibahagi ang impormasyon nang eksklusibo sa pamamagitan ng pribadong mensahe o ng mga sariling pamayanang grupo.
- Kapag binabanggit ang target na aplikasyon sa publiko, gumamit ng mga indirektang sanggunian (tulad ng "UM:PD" o "Ang Laro ng Kabayo") para maiwasan ang pag-index ng mga search engine.

**O ibahagi mo na lang at sirain mo ito para sa dosenang gumagamit ng Hachimi. Ikaw ang bahala.**

> [!WARNING]
> **Kung ibabahagi mo pa rin ito**
> Gawin mo ang kailangan mo, pero nakikiusap kaming subukan mong tawagin ang laro na "UM:PD" o "Ang Laro ng Kabayo" sa halip na ang tunay na pangalan nito, para maiwasan ang pag-parse ng mga search engine.

## Mga Tampok

- **Mataas na Kalidad na Lokalisasyon:** Sumusuporta sa advanced na pag-format ng teksto (mga plural form, ordinal na numero, dynamic na pagkasya sa layout) nang walang manu-manong pagbabago ng assets. Sinusuportahan din nitong isalin ang karamihan sa mga bahagi ng laro; walang kinakailangang manu-manong pag-patch ng assets!
  - Mga suportadong komponent:
    - Text ng UI
    - Mga entry sa database (`master.mdb`, pangalan at paglalarawan ng skill)
    - Mga kuwento ng karera at diyalogo ng pangunahing scenario
    - Mga liriko ng kanta
    - Dynamic na pagpapalit ng texture at sprite atlas
  - Nako-configure na sistema ng wika na sumusuporta sa mga custom na diksyunaryo ng lokalisasyon.
- **Awtomatikong Machine Translation (opsyonal):** Mga pagsasalin mula sa komunidad at machine para sa mga tekstong hindi pa sakop ng mga localization package, inilalapat nang direkta sa loob ng laro.
- **Configuration sa Loob ng Laro:** Naka-embed na GUI configuration editor na nagbibigay-daan sa real-time na pagbabago ng mga setting nang hindi nagre-restart ng aplikasyon.
- **Awtomatikong Update ng Lokalisasyon:** Ang built-in na updater ay nagda-download at nagre-reload ng mga napabantaying translation package nang direkta habang tumatakbo ang laro.
- **Pagpapahusay sa Graphics:** Mga tampok na pag-optimize ng device, kabilang ang pag-unlock ng target frame rate (FPS unlock) at resolution scaling.
- **Cross-Platform:** Native na suporta para sa Windows (DirectX 11 proxy DLL) at Android (Zygisk / Dobby inline hooks).
- **Mga Integration sa Windows Desktop:** Discord Rich Presence, mga control ng media ng Windows Media Transport (SMTC) na may live na jacket thumbnail, at pag-uulat ng progreso sa taskbar.

## Pag-install

Tingnan ang opisyal na [Getting Started Documentation](https://hachimi.noccu.art/docs/hachimi/getting-started.html).

## Pagbu-build mula sa Source

Ang detalyadong mga instruksyon sa compilation at pag-set up ng environment ay nasa dokumentong [BUILDING-fil.md](BUILDING-fil.md).

## Paalala sa Paggamit ng AI / LLM

Ang ilang bahagi ng Hachimi Edge —kabilang ang code, dokumentasyon, at nilalaman ng mga pagsasalin— ay nasulat o pinino sa tulong ng malalaking language model (LLM) at iba pang AI tool. Lahat ng output na tinulungan ng AI ay sinusuri bago isama, ngunit maaari pa rin itong maglaman ng mga hindi tumpak na impormasyon, maling salin, o hindi inaasahang pag-uugali. Gumamit nang may sariling paghatol; walang ibinibigay na garantiya ng kawastuhan para sa nilalamang gawa ng AI.

## Mga Kredit at Sanggunian

Ang Hachimi Edge ay gumagamit ng mga konsepto at teknik na itinatag ng mga sumusunod na open-source na proyekto:

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

## Lisensya

Ang proyektong ito ay lisensyado sa ilalim ng [GNU General Public License v3.0](../LICENSE).
