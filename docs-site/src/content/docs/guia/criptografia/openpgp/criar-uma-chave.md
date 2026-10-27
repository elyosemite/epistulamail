---
title: "Criar uma chave"
description: "Criar a sua chave OpenPGP."
sidebar:
  order: 1
---

Em **Configurações → Criptografia**, crie uma chave para a conta. Há três tipos:

- **Curve25519**: rápida, pequena e aceita pela maioria dos programas. É a recomendada.
- **RSA 4096**: para quem troca e-mail com programas antigos.
- **Pós-quântica (experimental)**: ML-KEM-768 com X25519. Poucos programas leem ainda.

:::caution[Faça o backup]
Sem backup, um computador perdido leva junto a chave, e as mensagens cifradas para ela não abrem mais. Use o backup com senha e guarde o arquivo fora do computador.
:::
