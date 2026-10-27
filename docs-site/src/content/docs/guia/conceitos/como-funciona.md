---
title: "Como o EpistulaMail funciona"
description: "O caminho de um e-mail do servidor até a sua tela."
sidebar:
  order: 1
---

O EpistulaMail é um programa de desktop feito com Tauri (Rust e React). Ele conversa direto com o servidor do seu e-mail, pelos protocolos de sempre:

- **IMAP** para baixar e organizar as mensagens;
- **SMTP** para enviar;
- **OAuth** para o login, sem a sua senha passar pelo app.

O que chega é guardado num banco local (SQLCipher, cifrado com AES-256). A lista, a leitura e a busca leem desse banco, por isso abrem rápido e funcionam sem internet. As mudanças que você faz (mover, arquivar, marcar como lida) vão para o servidor em seguida.

Não existe um servidor do EpistulaMail no meio: nenhuma mensagem, assunto ou endereço passa por nós.
