<div align="center">
  <img src="../assets/icon.png" width="128" height="128" alt="Hachimi Edge Logo">
  <h1>Hachimi Edge</h1>
  <p><b>Mod peningkatan dan terjemahan game untuk UM:PD</b></p>

  <p><a href="../README.md">English</a> | <a href="README-es.md">Español</a> | <a href="README-fil.md">Filipino</a> | Bahasa Indonesia | <a href="README-ja.md">日本語</a> | <a href="README-ko.md">한국어</a> | <a href="README-pt-br.md">Português (Brasil)</a> | <a href="README-ru.md">Русский</a> | <a href="README-vi.md">Tiếng Việt</a> | <a href="README-zh_cn.md">简体中文</a> | <a href="README-zh_tw.md">繁體中文</a></p>

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

## ⚠️ Pedoman Berbagi & Redistribusi

Proyek ini memodifikasi perilaku runtime game dan melanggar Ketentuan Layanan (TOS) aplikasi target. Sangat mungkin pengembang game ingin menghapus proyek ini begitu mengetahuinya. Untuk meminimalkan risiko bagi proyek dan penggunanya, harap patuhi pedoman berikut:

- **Jangan posting tautan langsung** ke repositori ini, situs web proyek, atau alat terkait di situs web, forum, atau media sosial publik.
- Bagikan informasi secara eksklusif melalui pesan pribadi atau grup komunitas yang dikelola sendiri.
- Saat menyebut aplikasi target di publik, gunakan referensi tidak langsung (seperti "UM:PD" atau "Game Kuda") untuk mencegah pengindeksan mesin pencari.

**Atau bagikan saja dan hancurkan untuk puluhan pengguna Hachimi. Terserah Anda.**

> [!WARNING]
> **Jika Anda tetap akan membagikannya**
> Lakukan saja, namun kami dengan hormat meminta Anda mencoba menyebut game sebagai "UM:PD" atau "Game Kuda" alih-alih nama aslinya, untuk menghindari penguraian mesin pencari.

## Fitur

- **Lokalisasi Berkualitas Tinggi:** Dukungan pemformatan teks tingkat lanjut (bentuk jamak, bilangan ordinal, penyesuaian tata letak dinamis) tanpa modifikasi aset manual. Juga mendukung penerjemahan sebagian besar komponen dalam game; tanpa perlu patching aset manual!
  - Komponen yang didukung:
    - Teks UI
    - Entri basis data (`master.mdb`, nama dan deskripsi skill)
    - Cerita balapan dan dialog skenario utama
    - Lirik lagu
    - Penggantian tekstur dan atlas sprite dinamis
  - Sistem bahasa yang dapat dikonfigurasi dengan dukungan kamus lokalisasi kustom.
- **Terjemahan Mesin Otomatis (opsional):** Terjemahan dari komunitas dan mesin untuk teks yang belum tercakup oleh paket lokalisasi, diterapkan langsung di dalam game.
- **Konfigurasi Dalam Game:** Editor konfigurasi GUI tersemat memungkinkan penyesuaian pengaturan secara real-time tanpa memulai ulang aplikasi.
- **Pembaruan Lokalisasi Otomatis:** Pembaru terintegrasi mengunduh dan memuat ulang paket terjemahan terbaru langsung saat game berjalan.
- **Peningkatan Grafis:** Fitur optimisasi perangkat termasuk membuka kunci target frame rate (unlock FPS) dan penskalaan resolusi.
- **Lintas Platform:** Dukungan native untuk Windows (proxy DLL DirectX 11) dan Android (inline hook Zygisk / Dobby).
- **Integrasi Desktop Windows:** Discord Rich Presence, kontrol media Windows Media Transport (SMTC) dengan thumbnail sampul live, dan pelaporan kemajuan di taskbar.

## Instalasi

Lihat [dokumentasi Memulai](https://hachimi.noccu.art/docs/hachimi/getting-started.html) resmi.

## Membangun dari Sumber

Instruksi kompilasi dan penyiapan lingkungan secara rinci didokumentasikan di [BUILDING-id.md](BUILDING-id.md).

## Pernyataan Penggunaan AI / LLM

Sebagian Hachimi Edge —termasuk kode, dokumentasi, dan konten terjemahan— telah ditulis atau disempurnakan dengan bantuan model bahasa besar (LLM) dan alat AI lainnya. Semua keluaran yang dibantu AI ditinjau sebelum disertakan, namun tetap dapat mengandung ketidakakuratan, kesalahan terjemahan, atau perilaku yang tidak diinginkan. Gunakan atas pertimbangan sendiri; tidak ada jaminan kebenaran untuk konten yang dihasilkan AI.

## Kredit & Referensi

Hachimi Edge menggabungkan konsep dan teknik yang ditetapkan oleh proyek-proyek sumber terbuka berikut:

- [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G)
- [umamusume-localify-android](https://github.com/Kimjio/umamusume-localify-android)
- [umamusume-localify](https://github.com/GEEKiDoS/umamusume-localify)
- [Carotenify](https://github.com/KevinVG207/Uma-Carotenify)
- [umamusu-translate](https://github.com/noccu/umamusu-translate)
- [frida-il2cpp-bridge](https://github.com/vfsfitvnm/frida-il2cpp-bridge)

## Lisensi

Proyek ini dilisensikan di bawah [GNU General Public License v3.0](../LICENSE).
