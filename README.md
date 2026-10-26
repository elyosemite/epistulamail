# Epistula

**English** | [Português](README.pt-BR.md)

A desktop email client that keeps your messages on your own computer, encrypted, with no server of ours in between.

Epistula connects your accounts, downloads what you need to read and stores it in a local database protected by AES-256. The key lives in your operating system's credential vault. There is no Epistula account, no Epistula cloud, and nobody on our side with access to your mail.

![Epistula inbox with three accounts, folders and labels, the message list and an open newsletter in the reading pane](docs/images/epistula-caixa-de-entrada.png)

> Epistula is under active development. Reading, sending, organizing, conversations, notifications, offline reading and multiple accounts work today. An app password, automatic lock and encrypted backups are on the way. [Current state](#current-state) lists exactly what exists right now.

## At a glance

- **No Epistula account, no Epistula cloud.** The app talks straight to your provider. There is no server of ours in between.
- **Everything on disk is encrypted.** Messages, subjects, senders and indexes live in an AES-256 database, which already holds up against quantum computers.
- **Opens instantly, even offline.** Recent mail is downloaded in the background and opens from disk.
- **Blocks tracking.** Remote images (the "spy pixels" that tell senders you opened their email) stay blocked until you allow them.
- **Conversations that read well.** Each reply in its own card, with quoted text tucked away.
- **Organizes itself.** Rules move, archive or flag mail as it arrives, with a live preview before you save and undo afterwards.
- **Tells you only what matters.** Windows notifications, a tray icon, quiet hours and a history behind the bell.
- **Security you can check.** A public threat model, and tests that fail if any content shows up in the logs.

## Why another email client

Email is the longest-running archive in most people's digital lives. Contracts, medical results, family threads, the password reset for everything else. And that archive usually lives in a single place, run by someone else, reachable through a browser tab.

Epistula starts from three simple ideas:

**Your data stays with you.** Every message you open is stored on your disk. If the network drops, your mailbox is still there. If your provider changes its rules, your copy is still yours.

**What sits on disk is encrypted.** The whole database is encrypted, including indexes, subjects and senders. A copied file, a laptop left in a cab or a discarded drive reveals nothing without the key.

**Security is shown, not promised.** The threat model is public, the risks we have not solved yet are written down, and the guarantees that can be tested have tests.

## Current state

What works today:

**Accounts and security**

- Google accounts over OAuth (PKCE), reading through IMAP and sending through SMTP, always over TLS. Your password never passes through the app.
- Several accounts at once, each with its own name, color and photo, one click apart.
- Local SQLCipher database (AES-256, per-page HMAC), with the key in the Windows Credential Manager and secure deletion of freed pages.
- Safe reading: third-party HTML is cleaned against an allowlist in the Rust process and shown in an iframe with scripts disabled. Remote images stay blocked until you allow them, one message at a time.
- Private notifications: you choose whether they show sender and subject or just the account.

**Reading**

- Conversations as cards: each reply on its own, unread ones open, quoted text hidden behind a "…".
- Offline reading: after every sync, mail from the period you pick (30 days, 90 days, everything, or only when opened) is downloaded in the background, without marking it as read.
- Instant opening: the app fetches the next messages in the list, the rest of the conversation and the one under your mouse ahead of time.
- Staged first sync: the list shows up with the 200 most recent messages, and the rest arrives gradually.

**Writing and organizing**

- Compose, reply, reply all and forward, with drafts saved encrypted.
- Folders and labels, flags with due dates, and move, archive, delete and mark as read for one message or many, including drag and drop onto a folder.
- Search, a command bar, and background sync with a per-account interval.
- Rules that organize mail as it arrives: move, archive, delete, label, mark as read, star, flag with a due date, silence or always notify. Each rule reads as a sentence, shows a live preview of the mail it would catch before you save it, and everything it does can be undone from its history. Start from a template or straight from a message ("always move mail from this sender").

**Staying informed**

- Windows notifications for new mail, replies to your emails, due dates and accounts that need attention. One click opens the message.
- The app stays in the system tray with the window closed, with "new email" and snooze.
- Quiet hours and a bell with the last 7 days of notices.

**Appearance**

- Light, dark or system theme, a customizable background color, list density, and collapsible columns to focus on reading.

What does not exist yet:

- Providers other than Google. The data model already covers Outlook, Yahoo, iCloud, Zoho, GMX, AOL and generic IMAP.
- Signed releases. For now, the way in is building from source.

## Where this is going

### Your data, only yours, even on your own computer

Today the database is encrypted against anyone outside your Windows session. The next step is to protect it from people inside it too: someone else using your session, the administrator of a corporate domain, or anyone holding a copy of the disk and your Windows password.

- **Epistula password (optional):** the database key gets wrapped by a key derived from your password with Argon2id. The password itself is never stored.
- **Automatic lock:** after a period of inactivity, or when Windows locks the screen, the app closes the database and wipes the key from memory.
- **Nothing leaks outside the database:** content-free notifications by default, no email in the embedded browser's cache, no crash dumps written to disk, and a heads-up when the disk has no BitLocker.
- **Keep less:** erase from your computer the mail outside your chosen period and the mail of accounts you signed out of, rotating the database key afterwards.
- **Honest links:** a warning when a link goes somewhere other than what it shows, and tracking parameters stripped.

### Ready for quantum computers

Data encrypted today can be captured today and decrypted years from now, once quantum computers can break the key exchanges in use. For email, which people keep for decades, that risk is not theoretical.

The encryption Epistula already uses on disk (AES-256) holds up in that scenario; there is no need to replace it. On the way to your provider, the app will offer the hybrid key exchange (X25519 with ML-KEM-768), which kicks in as soon as the server accepts it. And anything built on public keys (backups opened on another machine, sync between your own devices, end-to-end encrypted messages) will start out with hybrid schemes, so protection never rests on a single bet.

### Backups you control

Export a whole account to an encrypted file, keep it wherever you like (an external drive, a USB stick, the storage service of your choice) and restore it on another machine without downloading everything from the provider again. Only the key holder should be able to read it, and the format will be documented, so you are never locked into Epistula.

### Real email management

Full-text local search, offline included. Rules that also run on the server, with the app closed. Snoozing messages. A unified view across accounts. Signatures, templates and scheduled send. Other providers (Outlook, Yahoo, iCloud and generic IMAP). Sender authenticity indicators (SPF, DKIM, DMARC) shown while reading, to help spot phishing.

## How your information is handled

```
Your computer
├── Interface (WebView)      display only; never sees tokens or the database key
├── Rust process             OAuth, IMAP, SMTP, HTML cleaning, decides what reaches the screen
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

The full analysis, with the data inventory, the cryptography inventory, trust boundaries and the seventeen threats considered, is in [docs/arquitetura-e-ameacas.md](docs/arquitetura-e-ameacas.md) (in Portuguese).

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
src-tauri/src/       Rust backend (OAuth, IMAP, SMTP, sync, database, HTML cleaning, notifications)
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
