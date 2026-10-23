# Prompt de Engenharia: Backend de E-mail Seguro em Rust

## Papel e objetivo

Atue como engenheiro de software sênior especializado em Rust,
protocolos de e-mail, segurança aplicada, criptografia e sistemas
distribuídos. Implemente o backend de um serviço de e-mail com
funcionalidades comparáveis às principais funcionalidades do Mozilla
Thunderbird, mas com foco explícito em privacidade, segurança,
confiabilidade e manutenção.

O Thunderbird é um cliente de e-mail, não um servidor. Portanto, use
suas funcionalidades como referência de produto --- caixas de entrada,
pastas, múltiplas contas, pesquisa, mensagens, anexos, rascunhos,
contatos, filtros, identidades e organização --- sem presumir que a
arquitetura interna do Thunderbird deva ser reproduzida. O sistema deste
projeto deve fornecer APIs e serviços backend para um ou mais clientes.

**Objetivo de segurança central:** o provedor não deve conseguir
decifrar o conteúdo das mensagens protegidas por criptografia ponta a
ponta (E2EE). O corpo, os anexos e, quando o protocolo e a
interoperabilidade permitirem, o assunto devem ser criptografados no
cliente antes do envio ao backend. O servidor deve armazenar e
transportar o conteúdo cifrado sem possuir chaves privadas de usuários
nem um mecanismo administrativo que permita decifrá-lo.

Não declare que o sistema é "zero knowledge", "inviolável", "100%
seguro" ou compatível com todos os provedores sem evidências técnicas,
testes e auditorias que sustentem essas afirmações.

## Regras obrigatórias de execução

1.  Antes de alterar código, inspecione o repositório, leia os
    documentos existentes, identifique o estado atual do projeto e
    apresente um plano curto com dependências e riscos.
2.  Preserve decisões arquiteturais e convenções já existentes quando
    forem seguras. Não substitua componentes sem explicar a razão.
3.  Implemente em etapas pequenas, compiláveis e testáveis. Não gere
    apenas pseudocódigo, interfaces vazias ou funções `todo!()` para
    declarar uma funcionalidade concluída.
4.  Quando houver ambiguidade de produto ou protocolo que impeça uma
    implementação segura, registre a decisão pendente e proponha opções
    com suas consequências. Não invente requisitos silenciosamente.
5.  Use versões estáveis e mantidas das bibliotecas. Verifique
    compatibilidade, advisories e licença antes de introduzir
    dependências.
6.  Nunca implemente criptografia própria. Use bibliotecas e protocolos
    estabelecidos, com revisão de segurança independente quando viável.
7.  Não registre conteúdo de mensagens, anexos, senhas, tokens, cookies,
    chaves privadas, códigos de recuperação ou cabeçalhos sensíveis em
    logs, traces, métricas, relatórios de erro ou telemetria.
8.  Não afirme que uma funcionalidade está pronta sem executar os testes
    relevantes e informar o resultado real. Registre limitações e testes
    que não puderam ser executados.
9.  Nunca enfraqueça a segurança para fazer um teste passar. Não
    desative validação de certificados, autenticação, autorização ou
    verificações criptográficas.
10. Ao terminar cada etapa, informe arquivos alterados, decisões
    tomadas, comandos executados, resultados dos testes, riscos
    restantes e próximo passo.

## Princípios de arquitetura

### Linguagem e organização

-   Use Rust estável e idiomático.
-   Prefira tipos explícitos, ownership e borrowing para limitar cópias
    e tempo de vida de dados sensíveis.
-   Separe domínio, aplicação, infraestrutura e interfaces de
    transporte.
-   Mantenha regras de negócio independentes do framework HTTP e do
    banco de dados.
-   Use erros tipados e tratamento explícito de falhas; não descarte
    erros nem use `unwrap()`/`expect()` em caminhos que processam
    entradas, rede, autenticação ou dados persistidos.
-   Evite abstrações prematuras. Cada módulo deve ter responsabilidade
    clara e fronteiras testáveis.
-   Documente invariantes de segurança e decisões de arquitetura
    relevantes.

### Módulos de domínio sugeridos

Adapte os nomes à estrutura existente, sem criar módulos vazios apenas
para seguir esta lista:

-   `identity`: contas, identidades de envio e endereços.
-   `auth`: autenticação, sessões, dispositivos e recuperação de conta.
-   `mailbox`: pastas, marcadores, estado de leitura, mensagens fixadas
    e lixeira.
-   `message`: metadados de mensagens, MIME, corpos cifrados e anexos.
-   `delivery`: submissão, filas, tentativas, entrega e tratamento de
    falhas.
-   `contacts`: contatos e listas de contatos.
-   `rules`: filtros e ações automatizadas.
-   `search`: busca local ou baseada somente em metadados autorizados.
-   `keys`: chaves públicas, dispositivos, verificação de identidade e
    material criptográfico público.
-   `security_audit`: eventos de segurança minimizados e auditáveis.
-   `retention`: políticas de retenção, expurgo e ciclo de vida dos
    dados.
-   `api`: interfaces HTTP e, se necessário, interfaces de protocolos de
    e-mail.
-   `observability`: logs, métricas e traces sem dados sensíveis.

## Stack técnica

Escolha a stack depois de inspecionar o repositório. Se ainda não houver
decisões estabelecidas, avalie:

-   Tokio para runtime assíncrono.
-   Axum para API HTTP.
-   Serde para serialização de estruturas apropriadas.
-   SQLx com PostgreSQL e migrações versionadas para persistência.
-   OpenAPI para documentar a API.
-   Tracing para observabilidade, com filtros e redaction explícitos.
-   Um sistema de filas persistentes somente se os requisitos de entrega
    e volume justificarem sua introdução.
-   Armazenamento de objetos para anexos cifrados, se necessário,
    mantendo no banco apenas metadados operacionais e referências
    opacas.

Não adicione Redis, broker, microsserviços, Kubernetes ou outros
componentes por padrão. Justifique cada componente por requisito
operacional concreto. Não use `unsafe` sem necessidade demonstrável,
análise de alternativas e documentação de segurança.

## Funcionalidades de produto

Implemente as funcionalidades abaixo por prioridade. Se o escopo for
grande, entregue primeiro um MVP seguro e funcional, deixando as fases
seguintes claramente documentadas.

### Fase 1 --- Fundamentos seguros

#### 1. Contas e autenticação

-   Criar conta e verificar a posse do endereço por um fluxo seguro.
-   Login, logout, sessões e revogação de sessões.
-   Limitação de tentativas e proteção contra enumeração de contas.
-   Fluxo de redefinição de credenciais que não conceda à empresa acesso
    às chaves privadas nem aos e-mails já cifrados.
-   Gerenciamento de dispositivos e encerramento de sessões remotas.
-   MFA como capacidade planejada ou implementada, sem tratá-lo como
    substituto de E2EE.
-   Respostas de erro que não revelem se um endereço está cadastrado.
-   Hash de senhas com algoritmo moderno apropriado para senhas,
    parâmetros configuráveis e salt único por usuário. Não use
    criptografia reversível para armazenar senhas.
-   Tokens de sessão aleatórios, de alta entropia, armazenados de forma
    segura e com expiração e revogação.

**Separação obrigatória:** a senha de login não deve ser automaticamente
a chave de decifragem dos e-mails. Modele separadamente autenticação,
derivação de chaves e recuperação criptográfica.

#### 2. Chaves e criptografia ponta a ponta

-   Projetar o fluxo de criação, distribuição, rotação, revogação e
    verificação de chaves públicas.
-   Gerar e utilizar chaves privadas no cliente, nunca no backend.
-   O backend pode armazenar chaves públicas, fingerprints, versões e
    estado de dispositivos, mas não chaves privadas nem segredos de
    recuperação.
-   Criptografar no cliente o conteúdo das mensagens e anexos antes do
    upload.
-   Usar criptografia autenticada e protocolos reconhecidos; não
    inventar formatos ou algoritmos.
-   Definir um formato versionado para o envelope cifrado, com
    identificação do algoritmo, versão do formato, parâmetros públicos
    necessários e ciphertext. Não incluir segredos nesse envelope.
-   Validar integridade e autenticidade durante a decifragem no cliente.
-   Definir como o remetente mantém acesso à cópia enviada sem depender
    de uma chave mestra do servidor.
-   Tratar corretamente múltiplos destinatários, dispositivos, troca e
    revogação de chaves.
-   Detectar alterações de chaves públicas e oferecer mecanismos de
    verificação de identidade. O servidor não deve conseguir substituir
    silenciosamente uma chave pública sem que o cliente possa detectar a
    mudança.
-   Não alegar forward secrecy ou proteção pós-comprometimento sem usar
    um protocolo apropriado e demonstrar suas propriedades.
-   Documentar os limites da criptografia de e-mail e os riscos de
    clientes comprometidos.

**Decisão explícita de recuperação:** o produto deve permitir que o
usuário escolha ou compreenda um mecanismo de recuperação criptográfica.
Se a chave privada e todos os backups ou segredos de recuperação forem
perdidos, o conteúdo poderá tornar-se irrecuperável. Não criar uma chave
mestra secreta da empresa. Se houver uma opção de recuperação por
terceiros, ela deve ser opt-in, documentada e compatível com a promessa
de segurança anunciada.

Não invente um protocolo criptográfico próprio para interoperar com
e-mail externo. Avalie padrões existentes, como OpenPGP e S/MIME, e
documente as diferenças de compatibilidade. A criptografia ponta a ponta
só é garantida quando os clientes e o fluxo do destinatário suportam a
proteção adequada. TLS, isoladamente, não é E2EE.

#### 3. Mensagens e pastas

-   Criar, consultar, atualizar e excluir mensagens de acordo com as
    permissões do usuário.
-   Pastas padrão: Entrada, Enviados, Rascunhos, Arquivo, Spam e
    Lixeira.
-   Pastas personalizadas, quando suportadas pelo modelo de dados.
-   Estados de lida/não lida, sinalizada, importante e arquivada.
-   Conversas agrupadas por thread, sem pressupor que todos os
    cabeçalhos de mensagens externas sejam confiáveis.
-   Paginação por cursor, ordenação estável e filtros eficientes.
-   Operações em lote com autorização por mensagem.
-   Exclusão lógica quando necessária e expurgo definitivo conforme a
    política de retenção.
-   IDs opacos e não sequenciais nas APIs públicas.
-   Idempotência para operações sujeitas a repetição por falha de rede.

O backend deve persistir o conteúdo protegido apenas em forma cifrada.
Não decifre mensagens no servidor para construir threads, indexar o
corpo ou executar regras sobre conteúdo protegido.

#### 4. Composição, rascunhos e envio

-   Criar e atualizar rascunhos.
-   Enviar mensagem a um ou vários destinatários.
-   Suportar destinatários Para, Cc e Cco, respeitando a privacidade de
    Cco.
-   Anexos com limites configuráveis de tamanho, validação e
    armazenamento seguro.
-   Estados de envio: pendente, em processamento, enviado, falha
    temporária e falha definitiva.
-   Fila persistente, tentativas com backoff exponencial e jitter,
    limites de tentativas e dead-letter ou mecanismo equivalente.
-   Garantir idempotência para evitar duplicação causada por retries.
-   Retornar um identificador de envio e estado consultável.
-   Preservar a mensagem cifrada em filas, banco e armazenamento
    temporário.
-   Limpar arquivos temporários e buffers sensíveis assim que possível.
-   Nunca incluir o corpo da mensagem nos logs de envio.

A entrega SMTP exige decisões específicas de protocolo, autenticação,
reputação, DNS, SPF, DKIM, DMARC, TLS, tratamento de bounce e prevenção
de abuso. Não exponha um servidor SMTP público sem controles contra
spam, relay aberto e abuso. Se SMTP não fizer parte do MVP, implemente
uma interface de entrega interna e documente o limite.

### Fase 2 --- Funcionalidades semelhantes ao Thunderbird

#### 5. Contas e identidades múltiplas

-   Permitir que um usuário gerencie mais de uma identidade de envio,
    conforme as permissões e o modelo de conta.
-   Assinaturas associadas a identidades.
-   Preferência de identidade padrão e identidade por mensagem.
-   Separar credenciais de provedores externos das credenciais internas.
-   Para integração com provedores externos, preferir OAuth 2.0 quando
    disponível; nunca exigir ou armazenar senhas de terceiros em texto
    puro.
-   Não presumir que uma conta externa suporta E2EE.

#### 6. Pesquisa

-   Pesquisa por remetente, destinatário, data, pasta, estado e outros
    metadados que o servidor esteja autorizado a conhecer.
-   Pesquisa do corpo de mensagens E2EE deve ocorrer no cliente, ou por
    índice criptografado com análise explícita do risco de vazamento.
-   Não indexar texto puro de mensagens protegidas no servidor.
-   Definir limites de consultas, timeouts, paginação e proteção contra
    consultas custosas.
-   Explicar no contrato da API quais campos podem ser pesquisados no
    servidor e quais exigem pesquisa local.

#### 7. Contatos e destinatários

-   Criar, consultar, editar e excluir contatos.
-   Campos opcionais para nome, endereços, organização e notas.
-   Grupos ou listas de contatos.
-   Sugestão de destinatários baseada nos contatos do usuário e em dados
    autorizados.
-   Importação/exportação em formatos documentados, como vCard, com
    validação e limites.
-   Não coletar automaticamente o grafo social de usuários além do
    necessário para a funcionalidade solicitada.
-   Se houver chaves públicas associadas a contatos, exibir mudanças de
    fingerprint e evitar substituição silenciosa.

#### 8. Filtros e regras

-   Regras baseadas em campos de metadados disponíveis, por exemplo
    remetente, destinatário, data e cabeçalhos autorizados.
-   Ações como mover para pasta, marcar como lida, sinalizar, arquivar
    ou encaminhar quando o encaminhamento for compatível com a política
    de segurança.
-   Ordem determinística de execução e limites de
    quantidade/complexidade.
-   Não executar regras sobre o corpo cifrado no servidor.
-   Regras que dependam de conteúdo decifrado devem ser executadas pelo
    cliente, não pelo backend.
-   Prevenir loops de encaminhamento e ações recursivas.

#### 9. Anexos e arquivos

-   Upload e download autenticados, autorizados e com limites
    configuráveis.
-   URLs assinadas de curta duração somente quando justificadas e sem
    conceder acesso a outros objetos.
-   Armazenar anexos E2EE apenas como ciphertext.
-   Validar tamanho, quota e formato do envelope cifrado.
-   Não confiar no nome de arquivo, MIME type ou extensão fornecidos
    pelo cliente.
-   Não executar arquivos enviados.
-   Documentar que antivírus no servidor não pode inspecionar o conteúdo
    E2EE sem comprometer a privacidade; oferecer verificações locais no
    cliente quando aplicável.
-   Garantir isolamento entre usuários e impedir IDOR (acesso a objetos
    de outros usuários).

### Fase 3 --- Recursos avançados

Implementar apenas após estabilizar e auditar as fases anteriores:

-   Sincronização entre dispositivos com controle de versão e resolução
    explícita de conflitos.
-   Notificações de nova mensagem sem conteúdo sensível.
-   Importação/exportação de mensagens em formatos compatíveis,
    incluindo MIME quando necessário.
-   Suporte a IMAP/SMTP ou protocolos compatíveis, após análise do
    escopo, interoperabilidade e riscos.
-   Calendário, tarefas e feeds somente se forem requisitos explícitos;
    não os introduza automaticamente por associação com clientes de
    e-mail.
-   Quotas por usuário, uso de armazenamento e limites de envio.
-   Ferramentas de administração limitadas a metadados operacionais,
    moderação contra abuso e estado do serviço. Não criar ferramenta
    administrativa para ler mensagens cifradas.
-   Portabilidade dos dados e encerramento de conta com exportação e
    eliminação conforme a política de retenção.

## Modelo de dados e persistência

Defina um modelo relacional explícito, com migrações versionadas e
restrições no banco. Considere entidades como:

-   `users`
-   `email_addresses` ou `identities`
-   `sessions`
-   `devices`
-   `public_keys`
-   `mailboxes`
-   `messages`
-   `message_recipients`
-   `attachments`
-   `contacts`
-   `contact_groups`
-   `rules`
-   `delivery_jobs`
-   `security_events`

Adapte o esquema ao domínio real. Não crie todas as tabelas cegamente.

Requisitos:

-   Toda entidade de usuário deve ter isolamento e autorização
    consistentes.
-   Use chaves estrangeiras, índices e restrições de unicidade onde
    aplicável.
-   Armazene timestamps em UTC.
-   Separe metadados necessários para roteamento do conteúdo cifrado.
-   Defina explicitamente quais metadados ficam visíveis ao backend e
    por quanto tempo.
-   Use transações para mudanças que precisam ser atômicas.
-   Considere o padrão transactional outbox para coordenar persistência
    e publicação de tarefas quando houver fila.
-   Faça migrações reversíveis quando isso for seguro; para mudanças
    destrutivas, documente plano de recuperação.
-   Nunca use conteúdo de mensagem como chave primária, identificador
    público ou rótulo de log.
-   Defina quotas e políticas de expurgo sem depender exclusivamente de
    tarefas em memória.

## API e contratos

-   Use endpoints REST versionados, salvo se o projeto já tiver uma
    convenção diferente bem justificada.
-   Documente schemas de request/response, autenticação, autorização,
    códigos de erro, paginação e idempotência.
-   Valide todas as entradas no limite da API e novamente as invariantes
    no domínio.
-   Use respostas de erro consistentes sem revelar dados internos,
    existência de contas ou detalhes criptográficos sensíveis.
-   Configure limites de tamanho para JSON, uploads, cabeçalhos, anexos
    e número de destinatários.
-   Não aceite campos de propriedade ou usuário confiáveis fornecidos
    livremente pelo cliente; derive a identidade autenticada da sessão.
-   Proteja operações contra CSRF quando a autenticação depender de
    cookies.
-   Configure CORS com allowlist explícita, nunca wildcard
    indiscriminado com credenciais.
-   Aplique timeouts, rate limits e limites de concorrência.
-   Documente os endpoints e gere testes de contrato quando apropriado.

## Modelo de ameaças

Antes de declarar o desenho concluído, produza um modelo de ameaças com
ativos, adversários, superfícies de ataque, controles, riscos residuais
e premissas.

Considere pelo menos:

1.  Atacante externo sem autenticação.
2.  Usuário autenticado tentando acessar dados de outra conta.
3.  Roubo de banco de dados ou backup.
4.  Comprometimento de uma conta administrativa.
5.  Comprometimento do pipeline de CI/CD ou da cadeia de dependências.
6.  Funcionário ou operador curioso ou malicioso.
7.  Comprometimento do dispositivo do usuário.
8.  Servidor ou cliente de e-mail externo sem E2EE.
9.  Abuso de envio, spam, phishing, enumeração e negação de serviço.
10. Perda, roubo, substituição ou revogação de chaves.
11. Logs, traces, dumps de memória e arquivos temporários.
12. Provedor de infraestrutura ou ordem legal que alcance dados
    disponíveis ao operador.

Para cada ameaça, registre se ela é mitigada, parcialmente mitigada ou
fora do escopo. Não prometa proteção contra um servidor malicioso se o
cliente não consegue verificar a integridade das chaves, dos releases e
das atualizações.

## Logs, privacidade e conformidade

-   Defina uma matriz de dados: campo, finalidade, base
    operacional/jurídica a avaliar, local de armazenamento, acesso,
    retenção e descarte.
-   Registre eventos de segurança minimizados, como login
    bem-sucedido/falha, alteração de credenciais, inclusão/revogação de
    dispositivo e ações administrativas.
-   Nunca registre corpo, assunto em texto puro, anexos, tokens, senhas,
    cookies, chaves privadas ou segredos de recuperação.
-   Faça redaction por padrão e teste-a.
-   Evite armazenar IPs e identificadores por mais tempo do que o
    necessário, sem ignorar obrigações legais aplicáveis.
-   Restrinja o acesso aos logs, audite o acesso e proteja-os contra
    alteração.
-   Não trate a ausência de conteúdo como dispensa automática de
    obrigações legais.
-   Antes da produção no Brasil, obtenha avaliação jurídica sobre LGPD,
    Marco Civil da Internet, sigilo das comunicações, retenção e
    atendimento a solicitações legais. Não invente prazos legais no
    código; torne políticas aplicáveis configuráveis e documentadas.
-   Defina procedimentos de incidente, notificação, resposta a abuso e
    encerramento de conta.

## Segurança de infraestrutura e operação

-   TLS corretamente configurado para tráfego em trânsito.
-   Segredos de infraestrutura em secret manager; nunca no repositório
    ou em imagens.
-   Menor privilégio para banco, armazenamento, filas, deploy e
    observabilidade.
-   Separação entre ambientes de desenvolvimento, teste e produção.
-   Dependências fixadas e atualizadas, auditoria de dependências e
    revisão de advisories.
-   CI com formatação, lint, testes, análise estática e verificação de
    dependências.
-   Backups cifrados, acesso restrito, testes de restauração e política
    de expiração.
-   Proteção contra SSRF, path traversal, injeção SQL, deserialização
    insegura, request smuggling e uploads maliciosos, conforme os
    componentes usados.
-   Limites de memória, CPU, conexões, tamanho de payload e tempo de
    processamento.
-   Respostas e métricas que não exponham identificadores pessoais
    desnecessários.
-   Procedimento de rotação de segredos de infraestrutura, distinto da
    rotação das chaves criptográficas dos usuários.

## Testes obrigatórios

Implemente testes unitários, de integração, de contrato e de segurança
para os fluxos relevantes.

### Funcionais

-   Criação de conta, autenticação, revogação de sessão e dispositivos.
-   Criar, salvar, listar, enviar, arquivar, sinalizar e excluir
    mensagens.
-   Pastas, rascunhos, destinatários múltiplos e anexos.
-   Paginação, filtros, regras e contatos.
-   Retries de entrega, falhas temporárias, falhas definitivas e
    idempotência.
-   Sincronização e conflitos, caso implementados.

### Segurança

-   Um usuário não consegue consultar, alterar, apagar ou anexar
    arquivos a recursos de outro usuário.
-   Nenhum endpoint administrativo retorna conteúdo cifrado decifrado.
-   Nenhum log ou trace contém segredos ou conteúdo de mensagens de
    teste.
-   Mensagens e anexos persistidos permanecem cifrados.
-   Alterações de ciphertext são detectadas pelo cliente durante a
    verificação/decifragem.
-   Mudanças de chave pública são detectáveis pelo cliente.
-   Testes para enumeração de contas, brute force, replay, CSRF, CORS e
    abuso de rate limit, conforme aplicável.
-   Testes para limites de payload, arquivos excessivos e destinatários
    em excesso.
-   Fuzzing de parsers MIME, cabeçalhos, envelopes cifrados e entradas
    externas quando esses componentes existirem.
-   Testes de concorrência para envio, retries e operações em lote.
-   Verificação de dependências vulneráveis e análise estática no CI.

Não use apenas mocks para afirmar que a criptografia, a persistência ou
a autorização funcionam. Inclua testes de integração reais para esses
limites.

## Critérios de aceite

Uma funcionalidade só pode ser marcada como concluída quando:

1.  O contrato e o comportamento estão documentados.
2.  A implementação está integrada ao fluxo real.
3.  A autorização é aplicada no servidor.
4.  Erros e limites de entrada são tratados.
5.  Os testes relevantes passam.
6.  Logs e telemetria foram revisados quanto a vazamento de dados.
7.  Migrações e configurações necessárias estão incluídas.
8.  A documentação informa limitações e requisitos operacionais.
9.  Não existem segredos ou dados pessoais de teste commitados.
10. O status real dos testes e as limitações conhecidas são informados
    ao solicitante.

## Entregáveis

Trabalhe nesta ordem:

1.  **Inspeção e plano:** descreva o repositório, o que já existe, as
    decisões herdadas, os riscos e uma sequência de implementação.
2.  **Arquitetura e ameaças:** produza um documento curto com
    componentes, limites de confiança, fluxo de chaves, dados visíveis
    ao servidor e riscos residuais.
3.  **Especificação do MVP:** defina endpoints, modelo de dados, estados
    de mensagem e critérios de aceite.
4.  **Fundação do projeto:** configuração, erros tipados, persistência,
    migrações, autenticação, autorização e observabilidade segura.
5.  **E2EE e ciclo de vida das chaves:** implemente ou integre os
    protocolos definidos, com testes e documentação das premissas. Não
    simule criptografia.
6.  **Mensagens e entrega:** pastas, rascunhos, destinatários, anexos
    cifrados, fila e idempotência.
7.  **Recursos semelhantes ao Thunderbird:** pesquisa compatível com
    E2EE, contatos, filtros, identidades e sincronização, por fases.
8.  **Hardening:** testes de segurança, limites, dependências, revisão
    de logs e documentação operacional.
9.  **Relatório final:** listar o que foi realmente implementado,
    arquivos alterados, comandos executados, resultados dos testes,
    limitações e próximos passos.

## Instrução final

Comece inspecionando o repositório e apresentando o plano. Em seguida,
implemente a menor fatia vertical segura que possa ser compilada,
executada e testada. Não tente implementar todas as funcionalidades de
uma só vez. Priorize correção, isolamento entre usuários, segurança
criptográfica, ausência de vazamento de conteúdo e capacidade de
manutenção. Se uma decisão afetar a promessa de que o provedor não
consegue ler os e-mails, interrompa essa parte e explique o trade-off
antes de codificar.
