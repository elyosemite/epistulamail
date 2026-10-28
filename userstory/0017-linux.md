## User Story 17: EpistulaMail no Linux

**Como** usuário de Linux,
**quero** instalar e usar o EpistulaMail no meu computador, com as mensagens no disco cifradas e as chaves no cofre do sistema,
**para que** eu tenha a mesma privacidade e a mesma leitura offline que existem no Windows, sem depender de um servidor do EpistulaMail.

---

### 1. Contexto

Pedido em 27 out 2026. Até a 1.0.2 o app é distribuído só para Windows 10 e 11 (NSIS), e o cofre do sistema é o Gerenciador de Credenciais (DPAPI) pelo crate `keyring`. O Tauri 2, o React e o Rust já são multiplataforma; o SQLCipher e o OpenSSL são embutidos; o `keyring` 3.6.3 já compila com `sync-secret-service` e `crypto-rust`, e no Linux fala com o Secret Service (GNOME Keyring ou KWallet) por `dbus-secret-service` e libdbus. O que falta é build, empacotamento, tratamento do cofre ausente e a paridade essencial de recursos.

### 2. Critérios de aceite

**CA1. Build e pacotes**

- `pnpm tauri build` no Linux gera AppImage e `.deb` a partir de `src-tauri/tauri.linux.conf.json`, sem mudar o NSIS do Windows.
- O build não depende do Perl portátil do Windows: `OPENSSL_SRC_PERL` deixa de ser global e passa a ser definido só no fluxo Windows.
- O CI compila e testa no Linux (Ubuntu) e no Windows.

**CA2. Cofre do sistema**

- A chave do banco, os refresh tokens e o índice de contas continuam no cofre do sistema, sem ir para o banco, para arquivo ou para o log.
- No Linux, os itens usam o Secret Service com o mesmo serviço `epistula` (`epistula-dev` no build de desenvolvimento).
- Sem Secret Service disponível, o app mostra um aviso claro no idioma do sistema, explica como instalar ou iniciar o GNOME Keyring/KWallet e encerra sem apagar o banco.

**CA3. Paridade essencial**

- Espaço livre no disco para a importação de arquivos de e-mail no Linux.
- Raízes de confiança do S/MIME vindas do bundle de CAs do sistema no Linux.
- Aviso de disco sem cifra: BitLocker no Windows, LUKS no Linux; sem detecção, sem aviso.
- Bandeja do sistema com o menu e o tooltip de não lidas. O ponto de não lidas na barra de tarefas continua só no Windows.
- A senha do EpistulaMail, o bloqueio por inatividade, a troca de chave, o "esqueci a senha" e a importação funcionam como no Windows.

**CA4. Textos por sistema**

- A interface mostra "sistema" em vez de "Windows" onde o texto não é específico: tema, raízes de confiança, notificações e margem de disco.
- "Bloquear junto com o Windows" só aparece no Windows; no Linux, o bloqueio por inatividade continua.
- O aviso de disco cita BitLocker no Windows e LUKS no Linux.

**CA5. Atualização e distribuição**

- O AppImage se atualiza pelo `latest.json` com a entrada `linux-x86_64` e a assinatura do `.sig`.
- O `.deb` é atualizado pelo gerenciador de pacotes e isso fica escrito na documentação.
- A release publica os artefatos dos dois sistemas no mesmo rascunho.

**CA6. Site**

- O botão de download vira um botão dividido: a ação principal baixa o pacote recomendado para o sistema detectado e a seta à direita, dentro do mesmo botão, abre o menu com Windows, Linux AppImage e Linux `.deb`.
- O menu funciona por teclado e com leitor de tela, e o site continua sem script inline e sem requisição de terceiros.

**CA7. Documentação e ameaças**

- README, README.pt-BR, site, docs em pt/en/es/fr/it, changelog e `docs/arquitetura-e-ameacas.md` falam do Linux, do Secret Service e do que ainda não existe.
- O teste que falha se conteúdo aparecer nos logs ou no banco continua passando no Linux.

### 3. Interface

A interface reaproveita o design system. O botão dividido do site usa `.btn-split` e o menu usa `.menu` e `.menu-item`; a composição entra no DS como **L81**, com os estados aberto, foco, hover, desabilitado e os dois temas. As configurações ganham o comando `platform` para esconder o bloqueio do Windows no Linux e trocar os textos por sistema. Não há tela nova.

### 4. Decisões

- **D1. Cofre só com Secret Service.** Sem o cofre do sistema, o app avisa e encerra. Não há cofre em arquivo nesta entrega.
- **D2. AppImage e `.deb`.** O AppImage é o pacote com atualização automática; o `.deb` é instalado e atualizado pelo sistema.
- **D3. Paridade essencial.** Notificações nativas e bloqueio junto com a tela ficam para a US18.
- **D4. LUKS.** O aviso de disco cifrado no Linux usa `findmnt` e `lsblk`; se a detecção falhar, não há aviso.
- **D5. `platform`.** Um comando somente de leitura devolve `windows`, `linux` ou `macos`; a interface decide os textos e o que mostrar.
- **D6. Campo `withWindows`.** O nome do campo e a chave `lock_with_windows` no banco continuam, para não migrar dados; o controle só aparece no Windows.
- **D7. Atualização no Linux.** Só o AppImage se atualiza sozinho. O `.deb` não entra no `latest.json`.
- **D8. Assinatura.** Os pacotes Linux não têm assinatura de código; o SHA-256 continua publicado nas notas da versão.

### 5. Fora do escopo

- Notificações nativas, bloqueio junto com a tela, cofre em arquivo, `rpm`, Snap, Flatpak, AUR, macOS e o ponto de não lidas na barra de tarefas no Linux: ver a US18.
