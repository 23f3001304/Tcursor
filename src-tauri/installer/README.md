# Installer assets

Branding for the NSIS (`.exe`) and WiX (`.msi`) installers, wired in `tauri.conf.json`
(`bundle.licenseFile` + `bundle.windows.nsis` / `bundle.windows.wix`).

| File          | Size (px) | Config key            | Where it shows |
|---------------|-----------|-----------------------|----------------|
| `sidebar.bmp` | 164×314   | NSIS `sidebarImage`   | Welcome / Finish page left panel |
| `header.bmp`  | 150×57    | NSIS `headerImage`    | Top-right strip on interior pages |
| `dialog.bmp`  | 493×312   | WiX `dialogImagePath` | Welcome / Exit dialog background |
| `banner.bmp`  | 493×58    | WiX `bannerPath`      | Top banner on interior dialogs |

All are **24-bit BMP** (required by both installers). The hero panels use a deep-navy gradient so
the app icon pops; the WiX images keep their text side light because WiX draws its dialog text in
black.

They are generated from `../icons/icon.png` with ffmpeg (`gradients` background + `overlay` icon +
`drawtext` "TCursor" in Segoe UI Bold). To tweak, re-run the ffmpeg pipeline and re-export each at
the size above with `-pix_fmt bgr24`.

- `eula.rtf` — the license shown during install (a starter EULA; review before public release).
- `SIGNING.md` / `sign-dev.ps1` — how to remove the "Unknown Publisher" warning (code signing).
