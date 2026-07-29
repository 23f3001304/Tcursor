# Code signing (the "Unknown Publisher" / SmartScreen warning)

Windows shows **"Windows protected your PC"** / **"Unknown publisher – Run anyway"** for any
executable that is **not Authenticode-signed by a trusted certificate**. The publisher/copyright
metadata in `tauri.conf.json` does **not** remove this — only a real signature does.

There are two paths, depending on who runs the installer.

---

## A. Just your own machines (free, self-signed)

Run `sign-dev.ps1` (from this folder, in PowerShell):

```powershell
./sign-dev.ps1
```

It creates a self-signed **code-signing** certificate in your user store and prints its
**thumbprint**. Then, temporarily, point Tauri at it and build:

```jsonc
// tauri.conf.json  ->  bundle.windows
"windows": {
  "certificateThumbprint": "PASTE_THUMBPRINT_HERE",
  "digestAlgorithm": "sha256",
  "timestampUrl": "http://timestamp.digicert.com",
  "nsis": { /* ... */ },
  "wix":  { /* ... */ }
}
```

```powershell
npm run tauri build
```

To make the warning actually disappear **on a machine**, that machine must **trust** the cert
(import it into *Trusted Root Certification Authorities* + *Trusted Publishers*). `sign-dev.ps1`
prints the exact `Import-Certificate` commands but does **not** run them — trusting a certificate
is a security decision you should make deliberately.

⚠️ Self-signed helps only machines that trust your cert. It does **nothing** for other people who
download TCursor — their Windows has never heard of your cert.

> Do **not** commit `certificateThumbprint` or the generated `.pfx`/`.cer`. They are already
> covered by `.gitignore` in this folder.

---

## B. Public distribution (paid, real certificate)

Buy an **Authenticode code-signing certificate** from a CA (DigiCert, Sectigo, SSL.com, …):

- **OV (Organization Validated)** — cheaper. SmartScreen reputation builds up over time / installs.
- **EV (Extended Validation)** — pricier, ships on a hardware token, gives **instant** SmartScreen
  reputation. Best if you're distributing widely.

Then either:

1. **Store thumbprint** (OV cert imported into the Windows cert store): set
   `bundle.windows.certificateThumbprint` + `timestampUrl` + `digestAlgorithm` as in section A and
   `npm run tauri build` — Tauri signs both the `.exe` and the `.msi` automatically.

2. **Hardware token / custom flow** (typical for EV): use a custom sign command:

   ```jsonc
   "windows": {
     "signCommand": "signtool sign /tr http://timestamp.digicert.com /td sha256 /fd sha256 /sha1 <THUMBPRINT> %1"
   }
   ```

   (`%1` is the file Tauri passes in.)

Signing does **not** change any of the installer branding — the images, license, shortcuts, and
metadata all stay exactly as configured.

---

## C. Open source: free signing via SignPath Foundation

Because TCursor is open source, the **free** route is the **SignPath Foundation**
(`signpath.io/open-source`): a free OV code-signing certificate plus a cloud signing service for
qualifying OSS projects, driven from CI.

Reality check: SignPath's cert is **OV**, and Windows SmartScreen is *reputation-based*. A fresh OV
signature removes the "Unknown publisher" text and starts accruing reputation, but SmartScreen can
still warn on early downloads until enough signed installs build trust. Only a paid **EV** cert buys
*instant* reputation — that's a property of SmartScreen, not of any particular OV cert.

Steps:
1. Publish the repo with an OSI-approved license and apply at `signpath.io/open-source`.
2. Once approved, install the SignPath GitHub app and note your `organization-id`, project slug, and
   signing-policy slug.
3. Add them as repo secrets/vars and enable the signing step in `.github/workflows/release.yml`
   (already scaffolded, gated behind the `SIGNPATH_*` secrets).

Until then the workflow still builds and publishes **unsigned** installers, and the README can point
users to building from source (no SmartScreen) or "More info → Run anyway".

### Cheaper-than-EV alternatives
- **Certum Open Source Code Signing** (~$25–70/yr, hardware token) — OV, same reputation caveat.
- **Azure Trusted Signing** (~$10/mo) — OV, with some identity-verification hoops.
