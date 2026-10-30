# EpistulaMail — Your email, on your computer, encrypted

<p align="center">
  <img src="branding/epistulamail-logo-512.png" width="160" height="160" alt="EpistulaMail">
</p>

<p align="center">
  <a href="https://github.com/elyosemite/epistulamail/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/elyosemite/epistulamail/ci.yml?branch=main&style=flat-square&label=ci" alt="CI status"></a>
  <a href="https://github.com/elyosemite/epistulamail/releases/latest"><img src="https://img.shields.io/github/v/release/elyosemite/epistulamail?style=flat-square&label=release" alt="Latest release"></a>
  <a href="https://github.com/elyosemite/epistulamail/releases/latest"><img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux-0078D4?style=flat-square" alt="Windows | Linux"></a>
  <a href="https://www.epistulamail.com"><img src="https://img.shields.io/badge/site-epistulamail.com-1D7AFC?style=flat-square" alt="Site"></a>
</p>

<p align="center"><b>English</b> | <a href="README.pt-BR.md">Português</a></p>

EpistulaMail is an open-source desktop email client for Windows and Linux. It connects Google and personal Microsoft accounts over OAuth, keeps mail in an encrypted SQLCipher database on your computer, and opens old email archives (`.mbox`, `.pst`, `.msg`, `.eml`) with no account or network. There is no EpistulaMail account, cloud, telemetry or paid tier.

[Website](https://www.epistulamail.com) · [Docs](https://docs.epistulamail.com) · [Download](https://github.com/elyosemite/epistulamail/releases/latest) · [Changelog](CHANGELOG.md) · [Threat model](docs/arquitetura-e-ameacas.md) · [Design system](design-system/)

![EpistulaMail inbox with three accounts, folders and labels, the message list and an open newsletter in the reading pane](docs/images/epistula-caixa-de-entrada.png)

## Install

Download the latest release from [GitHub](https://github.com/elyosemite/epistulamail/releases/latest): `EpistulaMail_<version>_x64-setup.exe` for Windows 10/11 (64-bit), or the AppImage/`.deb` for Linux (it needs the Secret Service). The installer is not code-signed yet, so check the SHA-256 published with the release before running it. New versions install over the old one and keep accounts and messages.

## What it does

- Gmail over IMAP/SMTP, personal Outlook/Hotmail/Live over Microsoft Graph, and manual IMAP/SMTP accounts, all over TLS.
- Messages, indexes and settings in an encrypted local database; the key lives in the system credential vault.
- Third-party HTML is cleaned in Rust and remote images stay blocked until you allow them.
- Offline reading, conversations, local search, folders and labels, rules, OpenPGP and S/MIME, notifications and a tray icon.
- Opens `.mbox`, `.pst`, `.msg`, `.eml`, Maildir, Thunderbird and Apple Mail folders without an account.
- Not yet: Yahoo, iCloud, Zoho, GMX, AOL, POP3 and Outlook for Mac `.olm`; the installer is unsigned.

## Quick start

1. Connect a Google or personal Microsoft account, or open an email archive.
2. Recent mail appears first; the rest syncs in the background and stays readable offline.
3. Press `/` to search (`de:`, `para:`, `depois:`, `tem:anexo`).
4. In Settings, choose the offline period, set an app password and lock, and create rules.

While Google's verification is pending, Google sign-in is limited to the test users registered in the project; opening archives works for everyone.

## Build

Requirements: Node.js 24, pnpm 10, Rust 1.88+ and Tauri 2. On Windows run `./scripts/setup-build-tools.ps1`; on Linux install `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libdbus-1-dev`, `libxdo-dev`, `build-essential`, `pkg-config` and `perl`.

```powershell
pnpm install
Copy-Item .env.example .env   # Google and Microsoft OAuth clients
pnpm tauri dev
pnpm tauri build
```

For Microsoft, use a personal-account Entra app with the redirect `http://localhost:8400/oauth/callback`, public client flows on, and the delegated Graph scopes `Mail.ReadWrite`, `Mail.Send`, `User.Read`, `email`, `openid`, `profile` and `offline_access`. Put the client ID in `EPISTULA_MS_CLIENT_ID`; `.env.example` has the details.

## Tests

```powershell
pnpm test
cd src-tauri; cargo test
cargo clippy --all-targets -- -D warnings
```

## Layout

`src/` React interface · `src-tauri/` Rust backend · `design-system/` tokens and components · `site/` landing page · `docs-site/` user docs · `docs/` architecture and threat model.

## Contributing

Issues and pull requests are welcome in English or Portuguese. Read `AGENTS.md` and the threat model before changing credentials, storage or third-party content. For security issues, contact the maintainers privately instead of opening a public issue.

## License

To be defined.
