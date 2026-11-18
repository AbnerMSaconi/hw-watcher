# Third-Party Notices

Original code in this repository is licensed under the GNU General Public License v3.0 only (`LICENSE`, `NOTICE`).

## Slint

`hw-watcher-panel` uses [Slint](https://slint.dev) (`GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0`) under its `GPL-3.0-only` option (which is why this program is GPL-3.0-only too), with the software renderer and bundled fonts (`JetBrains Mono`, SIL Open Font License 1.1).

## NVAPI & BAR0 Register Offsets

- NVAPI interface IDs and `ClientVoltRailsGetStatus` struct layout follow [LACT](https://github.com/ilya-zlobintsev/LACT) (MIT License, Copyright (c) 2023 Ilya Zlobintsev).
- GA102 BAR0 VRAM (`0xE2A8`) and hotspot (`0x2046C`) read-only offsets follow [ThomasBaruzier/gddr6-core-junction-vram-temps](https://github.com/ThomasBaruzier/gddr6-core-junction-vram-temps).
