---
title: "Compilar"
description: "Rodar o EpistulaMail a partir do código."
sidebar:
  order: 1
---

Requisitos: Node.js 24, pnpm 10 e Rust.

```powershell
git clone https://github.com/elyosemite/epistulamail.git
cd epistulamail
pnpm install
./scripts/setup-build-tools.ps1
pnpm tauri dev      # desenvolvimento
pnpm tauri build    # instalador
```

O build de desenvolvimento usa uma pasta de dados e itens no cofre só dele, então não mexe no EpistulaMail instalado. Para conectar contas no desenvolvimento, crie os seus clientes OAuth e ponha no `.env` (veja o README).
