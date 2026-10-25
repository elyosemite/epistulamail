# Epistula

**English** | [Português](README.pt-BR.md)

A desktop email client that keeps your messages on your own computer, encrypted, with no server of ours in between.

Epistula connects your accounts, downloads what you need to read and stores it in a local database protected by AES-256. The key lives in your operating system's credential vault. There is no Epistula account, no Epistula cloud, and nobody on our side with access to your mail.

![Epistula inbox with several accounts, folders, the message list and an open message with remote images blocked](docs/images/epistula-caixa-de-entrada.png)

> Epistula is under active development. Reading, organizing and multiple accounts work today. Sending, full backups and post-quantum cryptography are on the way. [Current state](#current-state) lists exactly what exists right now.

## Why another email client

Email is the longest-running archive in most people's digital lives. Contracts, medical results, family threads, the password reset for everything else. And that archive usually lives in a single place, run by someone else, reachable through a browser tab.

Epistula starts from three simple ideas:

**Your data stays with you.** Every message you open is stored on your disk. If the network drops, your mailbox is still there. If your provider changes its rules, your copy is still yours.

**What sits on disk is encrypted.** The whole database is encrypted, including indexes, subjects and senders. A copied file, a laptop left in a cab or a discarded drive reveals nothing without the key.

**Security is shown, not promised.** The threat model is public, the risks we have not solved yet are written down, and the guarantees that can be tested have tests.

## Current state

What works today:

- Google accounts over OAuth (PKCE), reading mail through IMAP over TLS. Your password never passes through the app.
- Several accounts at once, each with its own name, color and photo, one click apart.
- Folders and labels, pagination, and background sync with a per-account interval.
- Local SQLCipher database (AES-256, per-page HMAC). The message list opens from disk and stays available offline.
- Safe reading: third-party HTML is cleaned against an allowlist in the Rust process and shown in an iframe with scripts disabled. Remote images stay blocked until you allow them, one message at a time.
- Move, archive, delete, mark as read and flag with a due date, for one message or many, including drag and drop onto a folder.
- Search, a command bar, and local drafts for reply, reply all and forward.
- Light, dark or system theme, and a customizable background color.

What does not exist yet:

- Sending mail (SMTP). Drafts are already saved, encrypted.
- Providers other than Google. The data model already covers Outlook, Yahoo, iCloud, Zoho, GMX, AOL and generic IMAP.
- Signed releases. For now, the way in is building from source.

## Where this is going

### Post-quantum cryptography

Data encrypted today can be captured today and decrypted years from now, once quantum computers can break the key exchanges in use. For email, which people keep for decades, that risk is not theoretical.

The symmetric encryption Epistula already uses at rest (AES-256) holds up in that scenario. The weak spot is everything built on public keys: backups that must be opened on another machine, sync between your own devices, and end-to-end encrypted messages. That is where hybrid schemes come in, pairing a classical algorithm with a standardized post-quantum one (X25519 with ML-KEM), so protection never rests on a single bet.

### Backups you control

Export a whole account to an encrypted file, keep it wherever you like (an external drive, a USB stick, the storage service of your choice) and restore it on another machine without downloading everything from the provider again. Only the key holder should be able to read it, and the format will be documented, so you are never locked into Epistula.

### Real email management

Full local search, offline included. Rules and filters. Snooze. A unified view across accounts. Signatures, templates and scheduled send. Sender authenticity indicators (SPF, DKIM, DMARC) shown while reading, to help spot phishing.

## How your information is handled

```
Your computer
├── Interface (WebView)      display only; never sees tokens or the database key
├── Rust process             OAuth, IMAP, HTML cleaning, decides what reaches the screen
├── OS credential vault      refresh tokens and the database key
└── epistula.db              messages, folders and settings, fully encrypted

        │ TLS
        ▼
Your email provider
```

A few things worth saying plainly:

- **Your provider can still read your email.** Epistula protects the local copy and the path to it. Until there is end-to-end encryption, whoever hosts the mailbox has access to it.
- **Logs contain no content.** No subject, no sender, no account address. A test walks through the whole app with unique markers and fails if any of them shows up in the logs or in plain text in the database file.
- **Signing out revokes access** at the provider and deletes the secret from the credential vault. Messages already downloaded stay on disk, encrypted, until there is an option to erase them.
- **A compromised computer stays compromised.** Malware running in your session can read whatever you can read. No email client fixes that, and we would rather say so than pretend otherwise.

The full analysis, with the data inventory, trust boundaries and the twelve threats considered, is in [docs/arquitetura-e-ameacas.md](docs/arquitetura-e-ameacas.md) (in Portuguese).

## Design system

An email client gets opened dozens of times a day, and every small bit of friction turns into real annoyance. So the interface comes from its own design system, in [`design-system/`](design-system/), with tokens, typography, colors, accessibility notes and a page for each component. It changes constantly, and every new screen goes through it before it becomes code.

Four principles guide the decisions: density with breathing room, reading first, one gesture per action, and state always visible. Suggestions about interface and usability are as welcome as code.

## Building

The main target today is Windows. Epistula is built with [Tauri 2](https://tauri.app), Rust and React.

Requirements:

- Node.js 24 and pnpm 10
- Rust 1.82 or later
- On Windows, the portable Perl used to compile SQLCipher's bundled OpenSSL. The script below downloads it and checks its SHA-256.

```powershell
pnpm install
./scripts/setup-build-tools.ps1
```

To connect a Google account, create an OAuth client of type "Desktop app" in Google Cloud and fill in `.env`:

```powershell
Copy-Item .env.example .env
# EPISTULA_GOOGLE_CLIENT_ID=...
# EPISTULA_GOOGLE_CLIENT_SECRET=...
```

Then:

```powershell
pnpm tauri dev      # development
pnpm tauri build    # installer
```

## Tests

```powershell
pnpm test                         # interface
cd src-tauri; cargo test          # backend, against a real SQLCipher database
cargo clippy --all-targets -- -D warnings
```

CI runs all of this on Windows on every push, plus a scan of the repository history for committed secrets.

## Layout

```
src/                 React interface
src-tauri/src/       Rust backend (OAuth, IMAP, sync, database, HTML cleaning)
src-tauri/migrations database schema versions
design-system/       visual components and tokens
docs/                architecture and threat model
userstory/           decision history, one story per delivery
```

Most internal documentation, user stories and commit messages are in Portuguese. Contributions in English are just as welcome.

## Contributing

Issues and pull requests are welcome, in English or Portuguese. Before opening a PR that touches credentials, storage or the display of third-party content, read the threat model: changes in those areas need to say which threat they affect and how they were tested.

Found a security issue? Please do not open a public issue. Reach the maintainers privately through GitHub and we will get back to you before any disclosure.

## License

To be defined.
