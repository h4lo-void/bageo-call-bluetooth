# Notices

Copyright (C) 2026 **h4lo-void** (<https://github.com/h4lo-void>).

Bageo is dual-licensed under the **GNU Affero General Public License v3.0 only**
(`AGPL-3.0-only`, verbatim text in [`LICENSE`](LICENSE)) together with the
Section 7 additional terms in [`LICENSING.md`](LICENSING.md), or a separate
written **Commercial License** from **h4lo-void** (see
[`COMMERCIAL_LICENSE.md`](COMMERCIAL_LICENSE.md)). General disclaimers and usage
notices are in [`LEGAL.md`](LEGAL.md).

The parts listed below come under other terms or from other projects.

## Files under other terms

- `crates/agent/data/opencc/*.txt` (`STCharacters.txt`, `STPhrases.txt`,
  `TSCharacters.txt`, `TSPhrases.txt`, `TWVariants.txt`,
  `TWVariantsPhrases.txt`, `TWVariantsRevPhrases.txt`) — unmodified Chinese
  conversion dictionary files from **OpenCC (Open Chinese Convert)** version
  `ver.1.4.2` (<https://github.com/BYVoid/OpenCC/tree/ver.1.4.2>), Copyright
  2010–2026 Carbo Kuo (`BYVoid`) and OpenCC contributors, licensed under the
  **Apache License, Version 2.0** (full text in
  [`LICENSES/Apache-2.0.txt`](LICENSES/Apache-2.0.txt)).
- `assets/ambience_*.wav` — built from CC0 (public-domain) recordings from
  Freesound plus sounds synthesised for this project. Sources are listed in
  [`assets/README.md`](assets/README.md).
- `assets/voices/*/*.wav` — 1,080 prerecorded voice preview and
  "cannot be reached" announcement clips generated for this project with
  `agent make-voice-samples` (see [`assets/README.md`](assets/README.md)).

## Third-party software

The Bageo program is built from Rust crates listed in `Cargo.lock`, and the
**Bageo Status** desktop tray app (`desktop/`) is built with Tauri 2 and Rust
crates listed in `desktop/Cargo.lock`, each distributed under its own
open-source license. The Bluetooth program (`bluetooth/`) installs BlueZ and
a few Python packages from the system's package sources; they stay under their
own licenses and are not part of this repository.

## Trademarks & third-party services

The names **Bageo**, **八哥** and **bageo-call**, together with the project's
visual marks, are reserved identifiers of this project; the open-source license
grants no trademark rights (AGPL-3.0 Section 7(e); see
[`LICENSING.md`](LICENSING.md)).

Bageo talks only to services you choose and configure with your own keys:
Google Gemini for the live voice, optionally another model for summaries, and
the notification services you add. Their terms and prices are strictly between
you and them. This project is not affiliated with or endorsed by Google LLC,
Apple Inc., the Bluetooth SIG or any of those services. "Android" and "Gemini"
are trademarks of Google LLC, "iPhone" is a trademark of Apple Inc., and
"Bluetooth" is a trademark of the Bluetooth SIG; they are used here solely to
identify compatible devices and APIs.

<!-- SPDX-FileCopyrightText: h4lo-void <https://github.com/h4lo-void/bageo-call> -->
