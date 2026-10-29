## Fatia 00: Decisões, rascunho do DS e modelo de ameaças

**Parent:** `userstory/0022-configuracao-manual-de-servidores.md`

**Tipo:** HITL

### O que construir

Fechar o desenho antes de qualquer código de produção:

- Registrar as decisões D1 a D10 da US22 e marcar como "Em aberto" só o que ainda depende de spike (D8 e D9).
- Criar `design-system/rascunho-us22.html` com o formulário completo de entrada e saída, nos temas claro e escuro.
- Atualizar `docs/arquitetura-e-ameacas.md` com os riscos novos: senhas manuais no cofre, host informado pelo usuário, detecção de STARTTLS e downgrade, fraquezas de CRAM-MD5/NTLM, dependências de GSSAPI, erros de autenticação e logs sem host/usuário/conteúdo.
- Definir a matriz de autenticação desta entrega e a matriz futura (envio avançado e POP3), sem prometer o que não funciona.

### Critérios de aceite

- [ ] O rascunho mostra formulário vazio, preenchido, testando, sucesso, erro de autenticação, erro de conexão, aviso de envio avançado indisponível e edição, nos dois temas.
- [ ] O rascunho usa só tokens e peças existentes; peças novas propostas têm número L e ficha.
- [ ] O rascunho passa `pnpm ds:check`.
- [ ] O modelo de ameaças cita os riscos novos e as mitigações; a tabela de ameaças e o inventário de criptografia são atualizados.
- [ ] As decisões D1 a D10 estão na US22; D8 e D9 explicam o que falta decidir e quem decide.
- [ ] A matriz de autenticação deixa explícito o que funciona na entrada, na saída e o que fica "Em breve".
- [ ] Nenhum código de produção é alterado nesta fatia.

### Bloqueada por

Nenhuma — pode começar imediatamente.
