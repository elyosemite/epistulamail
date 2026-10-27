---
title: "Versões e releases"
description: "Como uma versão nova é publicada."
sidebar:
  order: 3
---

1. `./scripts/release.ps1 <versão>` sobe a versão nos arquivos e abre a seção no `CHANGELOG.md`.
2. Commit, push e a tag `v<versão>`.
3. O pipeline compila, assina a atualização e cria a release em rascunho.
4. Publicada a release, o site e o aviso no sino seguem sozinhos.
