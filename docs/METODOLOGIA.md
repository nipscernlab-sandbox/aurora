# Metodologia: como a AURORA cresce

Este documento diz como o código da AURORA se organiza e como a API dele muda
sem quebrar quem a usa. Ele vale para quem vai acrescentar uma função nova,
mudar uma existente ou revisar a mudança de outra pessoa.

Ele não repete o que já está escrito em outro lugar. Preparar a máquina,
rodar os testes e as receitas das mudanças comuns estão em
[CONTRIBUTING.md](../CONTRIBUTING.md). A referência de cada função do Core
está em [API.md](API.md). A lista de comandos e de métodos do `aurora serve`
está em [CLI.md](CLI.md). O que é só do Studio está no
[docs/DEVELOPMENT.md](https://github.com/nipscernlab-sandbox/aurora-ide/blob/main/docs/DEVELOPMENT.md)
do repositório aurora-ide.

As seções 1 a 7 valem desde já. A seção 8 é a proposta do que muda no
código até a 1.0, para que um método novo custe menos arquivos do que custa
hoje.

## 1. A regra das camadas

O repositório tem uma biblioteca com toda a regra de negócio e várias
interfaces em volta dela. Cada camada só conhece a de baixo.

```
aurora-core          a regra: projeto, fluxos, ferramentas e erros
  aurora-learn       os exercícios do aurora learn, sobre o core
  aurora-installer   instalar, atualizar e desinstalar, sobre o core
  aurora-agent       o assistente de IA, sobre o core (experimental)
  aurora-git         o controle de versão: o git e o GitHub, sobre o core (experimental)
aurora-protocol      o contrato: pedidos, respostas e schemas, sobre o core, o aurora-learn e o instalador
aurora-cli           o binário aurora: os comandos de texto, o aurora serve (que fala também MCP), o aurora mcp e o aurora chat
editors/vscode     a extensão do VS Code, em TypeScript, pelo aurora serve ou pela CLI
aurora-ide         a AURORA IDE, no repositório próprio, com este como submódulo: o backend
                   (cliente do aurora serve, mais o que é só da IDE) e a interface em TypeScript
```

Cinco regras seguem dessa figura.

1. **A regra mora no core.** Toda decisão é uma função do `aurora-core`, com
   teste lá: o que rodar, em que ordem, o que validar, que arquivo escolher.
   Se um `if` de uma interface muda o que a AURORA faz, ele está no lugar
   errado.
2. **As interfaces só traduzem.** A CLI e o `aurora serve` abrem o projeto,
   chamam o core e mostram ou devolvem o resultado; o Studio e a extensão
   pedem ao `aurora serve` e mostram a resposta. Elas escolhem como mostrar,
   nunca o que fazer. O Studio não linka o core: o único crate da AURORA que
   ele linka é o `aurora-agent`, do assistente, até a 1.0.
3. **Uma camada não conhece a de cima.** O core não sabe que existe
   protocolo, CLI nem Studio. As mensagens dele não citam comandos: a dica
   com o comando que resolve é posta pela CLI, no campo `hint`.
4. **Uma capacidade nova nasce como método do protocolo.** Mesmo quando o
   primeiro cliente é o Studio, ela entra no `aurora-protocol` e no
   `aurora serve`. Assim chega à extensão, ao `aurora mcp` e a scripts sem ser
   escrita de novo.
5. **O TypeScript não copia tipo do Rust à mão.** Os tipos vêm de
   `docs/schema/`, gerados dos tipos Rust. O `aurora-types.ts` do Studio e
   o da extensão são gerados; o que se escreve à mão é só o que existe
   apenas numa interface.

## 2. O que é uma API da AURORA

Uma API da AURORA é um **método do protocolo**. Ele tem seis peças, sempre as
mesmas. O exemplo é o `history.compare`, que compara dois relatórios.

| Peça | Onde mora | No exemplo |
|---|---|---|
| Nome | a linha em `methods!` e o `#[serde(rename)]` da variante, em `crates/aurora-protocol/src/lib.rs` | `history.compare` |
| Pedido | uma variante de `Request`, no mesmo arquivo | `Request::HistoryCompare { project, id, against }` |
| Resposta | o tipo na linha de `methods!` e uma variante de `Response` que o carrega | `RunComparison`, em `Response::Comparison(Box<RunComparison>)` |
| Função | o módulo do domínio, no core (no `aurora-learn` ou no instalador, para `learn.*`, `update.*` e `install.*`) | `history::compare_reports` |
| Erros | as variantes de `AuroraError` que a função devolve, cada uma com `code()` | |
| Documentação | o `///` da variante e de cada campo | vira a descrição no schema e no `aurora mcp` |

Um comando da CLI não é uma API. Ele é uma interface para pessoas, com texto
e `--json`, e pode juntar vários métodos ou mostrar um só de outro jeito. O
contrato do `--json` da CLI continua valendo, como diz o CONTRIBUTING.md,
mas quem escreve um programa que fala com a AURORA usa o protocolo.

## 3. Nomes

| O quê | Regra | Exemplo |
|---|---|---|
| Método | `domínio.verbo` ou `domínio.verbo_objeto`, em minúsculas e em inglês | `project.status`, `history.plan_cleanup` |
| Variante de `Request` | o método em CamelCase | `HistoryPlanCleanup` |
| Função do core | `verbo_objeto`, no módulo do domínio | `history::compare_reports` |
| Ferramenta do `aurora mcp` | o método com `_` no lugar do ponto | `history_compare` |
| Função no frontend do Studio | `api.<domínio>.<verbo>`, em `src/ipc/api.ts` do aurora-ide, que pede o método pelo `aurora_call` | `api.history.compare` |
| Código de erro | `snake_case`, em inglês, e nunca muda depois de publicado | `experimental_disabled` |

Os domínios de hoje são `project`, `processor`, `flow`, `wave`,
`verilog`, `history`, `toolchain`, `experimental`, `fpga`, `update`,
`install`, `learn`, `git` e `github` (estes dois no crate `aurora-git`). Um domínio novo ganha um módulo próprio no core. Há
exceções antigas à tabela, como `fpga.config` e `wave.layout`, cujas
variantes são `FpgaConfigGet` e `WaveLayoutGet`. Elas ficam, mas não servem
de modelo.

## 4. O ciclo de vida de um método

Um método está sempre num de três estados.

- **Experimental.** Pode mudar ou sair em qualquer versão menor, sem o
  período de obsoleto. Antes da 1.0, todos os métodos estão aqui. Depois
  dela, todo método novo começa aqui até uma versão o declarar estável.
  Uma área inteira que ainda vai mudar muito, como a placa FPGA e o
  assistente de IA, fica também atrás de um recurso de
  `aurora_core::experimental`: desligado, o Studio esconde a área, e a CLI,
  o `aurora serve` e os fluxos do core (`flows::run`) recusam com
  `experimental_disabled`, pelo `experimental::require`.
- **Estável.** Só muda de forma compatível, nos termos da seção 5.
- **Obsoleto.** Continua respondendo por pelo menos uma versão menor
  inteira, com o substituto escrito na documentação e no CHANGELOG. Só sai
  numa versão maior, que também sobe o `PROTOCOL_VERSION`.

Antes da 1.0 tudo é experimental, então ainda não há o que marcar no código.
A marca entra com o registro dos métodos (seção 8, fase 3). Lançar a 1.0 é
o ato de marcar quais métodos ficam estáveis.

## 5. Compatibilidade e versões

Há três números de versão, e cada um diz uma coisa diferente.

| Número | Onde | Quando sobe |
|---|---|---|
| Versão da AURORA | os seis lugares de [RELEASE.md](RELEASE.md), seção 6.1 | a cada release, pelas regras da seção 2 de [RELEASE.md](RELEASE.md) |
| `PROTOCOL_VERSION` | `crates/aurora-protocol/src/lib.rs`, devolvido pelo `initialize` | só quando um cliente antigo deixaria de funcionar |
| Versão da extensão | `editors/vscode/package.json` | a cada `.vsix` publicado com mudança, para o editor reconhecer a atualização |

O que conta como compatível:

| Mudança | Compatível |
|---|---|
| Um método novo | sim |
| Um campo opcional novo no pedido, com padrão que mantém o comportamento de antes | sim |
| Um campo novo na resposta | sim |
| Uma variante nova num enum da resposta marcado `#[non_exhaustive]` | sim, e os clientes ignoram o valor que não conhecem |
| Um código de erro novo | sim |
| Um campo obrigatório novo no pedido | não |
| Tirar ou renomear um método, um campo ou uma variante | não |
| Mudar o tipo de um campo, o significado dele ou um valor padrão | não |
| Mudar um código de erro já publicado | nunca |

As mudanças compatíveis da tabela acrescentam algo ao contrato, e por isso
são novidade: entram numa versão menor, nunca numa versão de correção, com
uma linha no CHANGELOG ([RELEASE.md](RELEASE.md), seção 2). Uma
incompatível, antes da 1.0, também entra numa versão menor, com a linha no
CHANGELOG dizendo o que quebra e o `PROTOCOL_VERSION` somado de um. Depois
da 1.0, só numa versão maior, depois do período de obsoleto.

**O portão é o diff de `docs/schema/`.** Os arquivos de lá saem dos tipos
por geração e nunca são editados à mão. Toda revisão de uma mudança de
contrato lê esse diff e confere, com a tabela acima, se ela é compatível.

Um cliente descobre o que a AURORA sabe fazer pela lista de métodos que o
`initialize` devolve, não comparando versões. A extensão do VS Code ainda
compara versões quando usa a CLI direto, pela tabela `FEATURES` de
`editors/vscode/src/aurora/version.ts`. Uma função nova que depende da CLI
ganha uma linha lá.

## 6. Passo a passo de um método novo

Este é o caminho como ele está na 0.9.0. Cada passo diz o arquivo. A seção
8 propõe como encurtá-lo.

1. **Domínio e nome.** Escolha pela seção 3. Antes, confira se um campo
   opcional novo num método existente não resolve: um método a menos é um
   método a menos para manter.
2. **A função no core.** No módulo do domínio. Ela recebe o `&Toolchain`
   se roda ferramenta, o `&Project` ou o `&Processor`, as opções numa
   struct `XOptions` com `Default`, `Deserialize` e `JsonSchema`, e o
   `&Control` por último se roda ferramenta. Ela devolve
   `Result<T, AuroraError>`, com `T` em `Serialize` e `JsonSchema` e cada
   campo documentado. O teste fica no próprio módulo ou em
   `crates/aurora-core/tests/`. As regras do código do core estão no
   CONTRIBUTING.md, em "Regras do código".
3. **O protocolo.** Em `crates/aurora-protocol/src/lib.rs`: a variante de
   `Request`, com o `#[serde(rename = "domínio.verbo")]` e o `///` em cada
   campo; a variante de `Response`, se o tipo da resposta é novo; a linha
   em `methods!`, com a variante, o nome, o tipo do resultado e, entre
   colchetes, as marcações: `read` se só lê, `install` se mexe na
   instalação, `agent` se vira ferramenta de agente e `agent_update` se vira
   ferramenta só com `--allow-update`. Dela saem `METHODS`, `METHOD_INFO`,
   `result_schema`, `is_read_only` e `is_installation`. Sem essa linha, o
   crate não compila. O tipo do resultado vem do core, do `aurora-learn` ou do
   instalador, com `Serialize` e `JsonSchema`. O teste
   `the_method_list_matches_the_request_enum` confere `METHODS` contra o
   enum, e `every_result_has_a_shape_in_the_schema` recusa um resultado sem
   formato no schema (um `serde_json::Value`).
4. **O despacho.** Um braço em `Server::handle_with`, em
   `crates/aurora-cli/src/serve.rs`: abrir o projeto, chamar a função do core
   e embrulhar o resultado na `Response`. Nenhuma regra entra aqui: uma
   operação de mais de um passo vira uma função do core, que o braço e o
   comando da CLI chamam. O `match` não tem `_` (o `Request` não é
   `non_exhaustive`): sem o braço, o despacho não compila. O teste de ponta a
   ponta vai
   em `crates/aurora-cli/tests/serve.rs`. Se o método roda sem bundle, ele
   entra também em `results_conform_to_the_declared_schema`, nos testes de
   `crates/aurora-cli/src/serve.rs`, que confere o resultado contra o tipo da
   linha de `methods!`.
5. **O `aurora mcp`.** Se o método serve a um agente de IA, a linha dele em
   `methods!` leva `agent` (ou `agent_update`, se muda a instalação), e ele
   ganha uma linha em `TOOLS`, em `crates/aurora-cli/src/mcp/mod.rs`, com o
   nome da ferramenta e a posição na lista; o teste
   `the_tools_are_the_methods_marked_for_agents` confere as duas. Fica de fora o que
   abre janela, o que apaga sem um plano antes e o que só faz sentido numa
   interface com pessoa. O resumo em texto, em `mcp/summary.rs`, é
   opcional: sem ele, o agente recebe o JSON formatado.
6. **Schemas, referência e tipos gerados.** Gere de novo e leia o diff de
   `docs/schema/` e do `docs/PROTOCOL.md`, que sai do mesmo comando:

   ```bash
   AURORA_UPDATE_SCHEMA=1 cargo test -p aurora-cli --bin aurora schema
   ```

   ```bash
   cd editors/vscode && npm run gen:types
   ```

   Os tipos da IDE saem do mesmo schema, no repositório aurora-ide, quando
   o submódulo dele passa a apontar para o commit com o método (o
   DEVELOPMENT.md de lá, seção 6).

7. **As interfaces que vão usar o método.** Só as que precisam dele agora.
   - CLI: a receita "Acrescentar um comando ou uma opção à CLI" do
     CONTRIBUTING.md; o comando chama a mesma função do core que o braço.
   - Studio: no aurora-ide, a receita "Uma operação da AURORA na interface"
     do DEVELOPMENT.md de lá. A função do `api.ts` pede o método pelo
     `aurora_call`, e o backend não ganha comando. Chega aos usuários na
     release da IDE e, depois, na da AURORA que a leva no bundle.
   - Extensão do VS Code: o método na interface `AuroraClient`
     (`editors/vscode/src/aurora/client.ts`), em `serveClient.ts` e, se houver
     comando de CLI equivalente, em `cliClient.ts`.
8. **Documentação.** A função em [API.md](API.md); o método na lista do
   `aurora serve` em [CLI.md](CLI.md) (a referência dele, em
   [PROTOCOL.md](PROTOCOL.md), saiu no passo 6); a função do Studio na
   tabela do `aurora_call` do
   [IPC.md](https://github.com/nipscernlab-sandbox/aurora-ide/blob/main/docs/IPC.md)
   do aurora-ide, se o Studio a usa; uma linha no `CHANGELOG.md`, e no
   `CHANGELOG.md` do aurora-ide se o Studio mudou.

### Pronto quando

- [ ] A regra está no core, com teste que falha sem a mudança.
- [ ] O método está em `methods!`, no `Request`, no despacho e num teste de
      `crates/aurora-cli/tests/serve.rs`.
- [ ] A decisão sobre o `aurora mcp` foi tomada: na lista, ou de fora por um
      dos motivos do passo 5.
- [ ] `docs/schema/`, o `docs/PROTOCOL.md` e os dois `aurora-types.ts` foram
      gerados de novo, e o diff é compatível pela seção 5, ou a
      incompatibilidade está no CHANGELOG.
- [ ] Nenhuma interface decide o que o core já decide.
- [ ] A documentação e o CHANGELOG acompanham.
- [ ] As verificações de "Antes de mandar a mudança", do CONTRIBUTING.md,
      passaram.

### Mudar um método existente

Primeiro classifique a mudança pela tabela da seção 5. Se ela é compatível,
o caminho é o mesmo dos passos 2 a 8, só nas peças que mudam. Se não é,
prefira um método novo ao lado do antigo e marque o antigo como obsoleto.
Mudar a forma de um método em uso quebra todo cliente que já existe, e o
custo disso cai em quem não participou da decisão.

### Tornar obsoleto e tirar

1. Escreva no `///` da variante que o método está obsoleto e qual o
   substituto. A descrição vai para o schema e para o `aurora mcp`.
2. Uma linha no CHANGELOG da versão dizendo que o método ficou obsoleto e
   qual o substituto.
3. O método continua respondendo por pelo menos uma versão menor inteira.
4. Na versão maior seguinte, ele sai de todas as peças da seção 2, e o
   `PROTOCOL_VERSION` sobe.

## 7. Fluxo de trabalho e revisão

- **Branch.** Uma por assunto, a partir da `main`: `feat/<assunto>`,
  `fix/<assunto>` ou `docs/<assunto>`. A mudança chega na `main` por pull
  request.
- **Commit.** Um assunto por commit, com a mensagem em português e o
  prefixo da área: `Core:`, `CLI:`, `Studio:`, `VS Code:`, `Instalador:`,
  `Learn:`, `Agente:`, `Tipos gerados:` ou `Docs:`. A mudança de contrato e
  a geração dos tipos podem ir em commits separados, para o diff de
  `docs/schema/` ser lido sozinho.
- **Descrição do pull request.** O que muda, por quê, como foi testado e se
  muda o contrato: nenhuma mudança, compatível ou incompatível.
- **Quem revisa o contrato.** Uma mudança no `aurora-protocol`, nos códigos de
  `error.rs` ou em `docs/schema/` passa pela revisão de quem cuida do
  contrato. Até o `CODEOWNERS` da seção 8 existir, isso é combinado no pull
  request.
- **Quem submete responde pelo código.** Vale também para código escrito
  por um assistente de IA, como o CONTRIBUTING.md já diz.

## 8. O que muda no código até a 1.0

Esta seção é uma proposta. A fase 2 está feita, de outro jeito que o
proposto aqui; as outras, não.

### Onde está o custo hoje

- **Um método mexe em muitos arquivos.** O `history.compare` aparece em 14
  arquivos, fora os tipos gerados. Um método usado por todos os clientes
  passa por uns 15: core, protocolo, despacho, `aurora mcp`, CLI, Studio,
  extensão e documentação.
- **Um campo de resposta ainda sem tipo.** O `learn` do `FlowOutcome`, a
  correção que o `flow.run` do fluxo `learn` devolve, é um JSON solto: o
  `FlowOutcome` é do core, e o core não conhece o `Grade` do `aurora-learn`.
- **Os testes Rust não rodam no GitHub.** O `release.yml` só monta os
  instaladores, e o `vscode.yml` só cobre a extensão. O `ci.yml`, que os
  rodava nos três sistemas, saiu em 2026-10-06.

### As fases

Cada fase vale sozinha e pode parar ali.

1. **A rede de segurança.** Um `cargo fmt --all` do repositório inteiro,
   num commit só e sem outra mudança, porque hoje dez arquivos estão
   fora do `rustfmt`. Depois, CI a cada push e pull request, nos três
   sistemas: `cargo fmt --check`, `clippy`, os testes sem bundle, o
   `cargo doc`, a conferência de `docs/schema/` e o typecheck dos dois
   projetos TypeScript. Um `CODEOWNERS`
   para o `aurora-protocol`, `error.rs` e `docs/schema/`, e um modelo de pull
   request com as perguntas da seção 7.
2. **Um despacho só.** Feita em 2026-10-26, sem o crate `lace-service`
   que esta seção propunha: o Studio virou cliente do `aurora serve`, como a
   extensão, e deixou de linkar o core, o `aurora-learn` e o instalador. O
   despacho é um só, o de `crates/aurora-cli/src/serve.rs`, e o `aurora serve`
   e o `aurora mcp` são a mesma porta (o primeiro pedido diz se a sessão é do
   protocolo da AURORA ou MCP). No Studio, os comandos que repetiam o
   despacho deram lugar ao `aurora_call`; ficaram próprios os comandos que
   são do Studio: as operações longas com canal, os arquivos, o terminal, o
   vigia de arquivos, as preferências e a janela. A CLI chama direto as
   mesmas funções do core que o despacho chama; as operações de mais de um
   passo (registrar arquivos, conferir um fluxo antes do bundle, a visão de
   um relatório) são funções do core, para a CLI e o despacho não as
   repetirem. Depois, o protocolo saiu do core para um crate próprio, o
   `aurora-protocol`, acima do core, do `aurora-learn` e do instalador, e as
   respostas do update, do install e do learn ganharam tipo no schema.
3. **O método declarado num lugar só.** Uma macro, como a `reports!` da
   CLI, em que cada método declara uma vez o nome, o estado da seção 4, se
   só lê, se é da instalação, se vai para o `aurora mcp` e o resumo.
   Começada: a `methods!` do `aurora-protocol` declara o nome, o tipo do
   resultado e as marcações (`read`, `install`, `agent`, `agent_update`);
   `METHODS`, `METHOD_INFO`, `result_schema`, `is_read_only` e
   `is_installation` saem dela; uma variante de `Request` sem linha não
   compila; o despacho não tem mais o braço `other =>`, e o compilador
   exige o braço de cada método; e um teste confere a lista do `aurora mcp`
   contra as marcações. Falta o estado da seção 4 e o resumo para os
   agentes.
4. **O contrato testado e a compatibilidade conferida pela máquina.** Casos
   de conformidade, pares de pedido e resposta em JSON, rodados contra o
   `aurora serve` e contra os clientes em TypeScript. Na CI, uma comparação de
   `docs/schema/` com a última release que classifica cada mudança pela
   tabela da seção 5 e falha numa incompatível sem o `PROTOCOL_VERSION`
   somado.

Depois da fase 3, um método novo custa isto:

| Peça | Hoje | Depois da fase 3 |
|---|---|---|
| Função e teste no core | sim | sim |
| Variante em `Request` e `Response` | sim | sim |
| `METHODS`, `is_read_only`, `is_installation` | saem de `methods!` | saem do registro |
| Braço no despacho | no `aurora serve`, exigido pelo compilador | no `aurora serve`, exigido pelo compilador |
| Linha no `aurora mcp` | a marcação em `methods!` e o nome em `TOOLS`, conferidos por teste | sai do registro |
| Comando no Studio | nenhum, pelo `aurora_call` (desde a fase 2) | nenhum |
| Tipos em TypeScript | gerados | gerados |
| Documentação e CHANGELOG | sim | sim |

A ordem sugerida é a das fases. A fase 1 é pequena e protege as outras. A
fase 2, a que mais simplificava o Studio, está feita. A fase 3 depende da
2, e a 4 depende da 1.
