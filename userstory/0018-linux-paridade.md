## User Story 18: Paridade futura do Linux e outros sistemas

**Como** usuário do EpistulaMail em Linux e, no futuro, em macOS,
**quero** as notificações, o bloqueio junto com a tela, um cofre alternativo e os pacotes das outras distribuições,
**para que** a experiência seja a mesma do Windows em qualquer sistema, sem perder privacidade nem conteúdo.

---

### 1. Contexto

A US17 entrega o Linux com o essencial: AppImage, `.deb`, Secret Service, espaço livre, raízes de confiança, aviso de LUKS, bandeja e os textos por sistema. O que ficou de fora foi registrado aqui para não se perder: notificações nativas, bloqueio junto com a tela, cofre em arquivo quando o Secret Service faltar, os outros formatos de pacote e o suporte ao macOS.

### 2. Critérios de aceite

**CA1. Notificações nativas no Linux**

- E-mail novo, resposta, prazo e conta que precisa de atenção aparecem como notificação do sistema quando a janela está fechada ou na bandeja.
- O clique abre a conta e a mensagem, como no Windows.
- O horário de silêncio e o "só a conta" continuam valendo.
- Sem notificação nativa disponível, o sino e o histórico continuam funcionando e a interface diz isso.

**CA2. Bloqueio junto com a tela**

- No Linux, quando a sessão bloquear (logind ou o protetor de tela), o app bloqueia junto se a senha do EpistulaMail estiver ligada e a opção ativa.
- No macOS, o equivalente for possível.
- A opção "Bloquear junto com o Windows" vira "Bloquear junto com o sistema" e aparece em todos os sistemas que suportam.

**CA3. Cofre alternativo**

- Quando o Secret Service não existir, o app oferece um cofre em arquivo cifrado com Argon2id e AES-256-GCM, exigindo a senha do EpistulaMail.
- O arquivo fica na pasta de dados, com permissão só do usuário, escrita atômica e testes de senha errada, corrupção e "esqueci a senha".
- Sem a senha, o banco não abre e os tokens não dão acesso ao servidor.
- O modelo de ameaças é atualizado com o novo formato.

**CA4. Outros pacotes**

- `rpm` para Fedora e openSUSE.
- Flatpak com o portal do Secret Service.
- Snap, se o Secret Service funcionar no confinamento.
- AUR com o pacote do AppImage ou do `.deb`.
- A documentação diz qual formato se atualiza sozinho e qual depende do sistema.

**CA5. Barra de tarefas no Linux**

- O ponto de não lidas aparece no ícone da bandeja ou na barra de tarefas quando a plataforma permitir.
- O tooltip e o menu da bandeja continuam iguais.

**CA6. macOS**

- Build, pacote `.dmg`, cofre no Keychain, raízes de confiança do sistema, aviso de disco cifrado e atualização automática.
- A interface e a documentação tratam o macOS como sistema de primeira classe.

### 3. Interface

As notificações seguem o texto e a privacidade da US8. O bloqueio junto com o sistema usa o mesmo ajuste da US10, agora com o nome por sistema. O cofre alternativo precisa de uma tela de definição de senha aprovada no design system antes de ir para o app. Os pacotes novos não mudam a interface.

### 4. Decisões

- **D1. Notificações.** Implementar por `notify-rust` ou `tauri-plugin-notification`, com ações de clique quando o servidor de notificações suportar.
- **D2. Bloqueio de sessão.** Preferir o sinal de sessão do logind no Linux, com alternativa pelo protetor de tela do ambiente.
- **D3. Cofre alternativo.** Só entra se o Secret Service continuar ausente em uma parte relevante das distribuições; a senha do EpistulaMail passa a ser obrigatória nesse modo.
- **D4. Flatpak e Snap.** Dependem do portal do Secret Service; validar antes de publicar.
- **D5. macOS.** História separada, com o Keychain no lugar do Secret Service.

### 5. Fora do escopo

- Celular (Android e iOS).
- Sincronização entre dispositivos.
- Servidor de login próprio.
