# EpistulaMail — Seu e-mail, no seu computador, cifrado

<p align="center">
  <img src="branding/epistulamail-logo-512.png" width="160" height="160" alt="EpistulaMail">
</p>

<p align="center">
  <a href="https://github.com/elyosemite/epistulamail/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/elyosemite/epistulamail/ci.yml?branch=main&style=flat-square&label=ci" alt="Situação do CI"></a>
  <a href="https://github.com/elyosemite/epistulamail/releases/latest"><img src="https://img.shields.io/github/v/release/elyosemite/epistulamail?style=flat-square&label=release" alt="Última versão"></a>
  <a href="https://github.com/elyosemite/epistulamail/releases/latest"><img src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux-0078D4?style=flat-square" alt="Windows | Linux"></a>
  <a href="https://www.epistulamail.com"><img src="https://img.shields.io/badge/site-epistulamail.com-1D7AFC?style=flat-square" alt="Site"></a>
</p>

<p align="center"><a href="README.md">English</a> | <b>Português</b></p>

O EpistulaMail é um cliente de e-mail desktop de código aberto para Windows e Linux. Ele conecta contas Google e Microsoft pessoais por OAuth, guarda as mensagens num banco SQLCipher cifrado no seu computador e abre arquivos de e-mail antigos (`.mbox`, `.pst`, `.msg`, `.eml`) sem conta e sem rede. Não existe conta EpistulaMail, nuvem, telemetria nem plano pago.

[Site](https://www.epistulamail.com) · [Documentação](https://docs.epistulamail.com) · [Download](https://github.com/elyosemite/epistulamail/releases/latest) · [Mudanças](CHANGELOG.md) · [Modelo de ameaças](docs/arquitetura-e-ameacas.md) · [Design System](design-system/)

![Caixa de entrada do EpistulaMail com três contas, pastas e marcadores, a lista de mensagens e uma newsletter aberta no painel de leitura](docs/images/epistula-caixa-de-entrada.png)

## Instalar

Baixe a última release no [GitHub](https://github.com/elyosemite/epistulamail/releases/latest): `EpistulaMail_<versão>_x64-setup.exe` para Windows 10/11 (64 bits), ou o AppImage/`.deb` para Linux (precisa do Secret Service). O instalador ainda não é assinado, então confira o SHA-256 publicado na release antes de executar. Versões novas instalam por cima e mantêm as contas e as mensagens.

## O que faz

- Gmail por IMAP/SMTP, Outlook/Hotmail/Live pessoal pelo Microsoft Graph e contas IMAP/SMTP manuais, sempre sobre TLS.
- Mensagens, índices e ajustes num banco local cifrado; a chave fica no cofre do sistema.
- O HTML de terceiros é limpo em Rust e as imagens externas ficam bloqueadas até você liberar.
- Leitura sem conexão, conversas, busca local, pastas e marcadores, regras, OpenPGP e S/MIME, notificações e bandeja.
- Abre `.mbox`, `.pst`, `.msg`, `.eml`, Maildir, Thunderbird e pastas do Apple Mail sem conta.
- Ainda não: Yahoo, iCloud, Zoho, GMX, AOL, POP3 e `.olm` do Outlook para Mac; o instalador não é assinado.

## Primeiros passos

1. Conecte uma conta Google ou Microsoft pessoal, ou abra um arquivo de e-mail.
2. Os e-mails mais recentes aparecem primeiro; o resto sincroniza em segundo plano e fica legível sem conexão.
3. Aperte `/` para buscar (`de:`, `para:`, `depois:`, `tem:anexo`).
4. Em Configurações, escolha o período offline, crie uma senha e bloqueio do app, e monte as suas regras.

Enquanto o Google verifica o app, o login com Google fica limitado aos usuários de teste cadastrados no projeto; abrir arquivos funciona para todos.

## Compilar

Requisitos: Node.js 24, pnpm 10, Rust 1.88+ e Tauri 2. No Windows, rode `./scripts/setup-build-tools.ps1`; no Linux, instale `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libdbus-1-dev`, `libxdo-dev`, `build-essential`, `pkg-config` e `perl`.

```powershell
pnpm install
Copy-Item .env.example .env   # clientes OAuth do Google e da Microsoft
pnpm tauri dev
pnpm tauri build
```

Para a Microsoft, use um app do Entra para contas pessoais com o redirect `http://localhost:8400/oauth/callback`, fluxos de cliente público ligados e os escopos delegados do Graph `Mail.ReadWrite`, `Mail.Send`, `User.Read`, `email`, `openid`, `profile` e `offline_access`. Coloque o client ID em `EPISTULA_MS_CLIENT_ID`; o `.env.example` tem os detalhes.

## Testes

```powershell
pnpm test
cd src-tauri; cargo test
cargo clippy --all-targets -- -D warnings
```

## Estrutura

`src/` interface em React · `src-tauri/` backend em Rust · `design-system/` tokens e componentes · `site/` landing page · `docs-site/` documentação do usuário · `docs/` arquitetura e modelo de ameaças.

## Contribuindo

Issues e pull requests são bem-vindos em português ou inglês. Leia o `AGENTS.md` e o modelo de ameaças antes de mexer em credenciais, armazenamento ou conteúdo de terceiros. Para falhas de segurança, fale com os mantenedores em privado em vez de abrir uma issue pública.

## Licença

A definir.
