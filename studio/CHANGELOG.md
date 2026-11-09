# Mudanças

O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/);
as versões, o [SemVer](https://semver.org/lang/pt-BR/).

## [Não lançado]

### Acrescentado

- **Controle de versão**, experimental (Preferências > Experimental >
  Controle de versão): a vista **Controle de versão** (Ctrl+Shift+G), no
  modelo do GitHub Desktop, com as abas Mudanças e Histórico, o seletor de
  ramo (trocar, criar, trazer um remoto), o botão que publica, envia, traz
  ou busca, o commit dos arquivos marcados, o descarte de um arquivo (para a
  lixeira), a criação do repositório e a identidade dos commits; e a vista
  **Perfil**, com o login no GitHub pelo código de dispositivo e a foto da
  conta no ícone. Tudo abre dentro das vistas, que vão para a barra lateral
  esquerda ou para a direita. A comparação de um arquivo abre numa aba de
  diff do editor; nas mudanças do disco, o lado novo é o próprio arquivo,
  editável. O Explorador marca os arquivos mudados (a letra e a cor da
  mudança, e um ponto na pasta), a barra de status mostra o ramo com os
  commits por enviar e por trazer, e o vigia da pasta passa a avisar do
  `HEAD`, do índice e das referências do `.git`, para um commit feito no
  terminal aparecer na vista.
  O remoto aparece embaixo do ramo e abre o formulário que corrige o
  endereço ou tira o remoto; quando publicar, enviar ou trazer falha porque
  o endereço está errado ou a conta não tem acesso, o formulário abre
  sozinho, com o que o git disse, e salvar tenta de novo. Arquivos de fora
  da pasta do projeto (o projeto dentro de um repositório maior) chegam
  desmarcados para o commit.
- Um aviso pode ter um botão que faz o que ele sugere (entrar no GitHub,
  trazer os commits).
- O erro do servidor da AURORA chega à interface com os campos dele
  (`IpcError.data`).

- A aba **Placa virtual** (Fluxo > Mostrar a placa virtual), com a placa
  FPGA ligada nas Preferências: a placa do projeto desenhada a partir do
  JSON dela, com chaves e botões clicáveis, LEDs, displays de 7 segmentos,
  tela, teclado e serial, ligada ao `vboard.run` do servidor por um canal
  próprio, fora da fila de operações. Em cima, o monitor, a serial e o
  teclado que o topo usa; embaixo, a placa, com os LEDs sobre as chaves de
  mesmo bit, os botões e os displays, e a porta do topo de cada elemento em
  faixas.
- **Editar ligações** na placa virtual: com a placa desligada, um clique
  numa chave, num botão, num LED ou num display escolhe a porta do topo que
  vai nele (`fpga.connect`), e o `fpga.json` é gravado.
- A tabela de ligações da aba Placa FPGA lê e grava a saída que vai a
  vários sinais da placa (uma linha para cada, uma lista no `fpga.json`).
- Botão **Placa virtual** na barra de ferramentas clássica, ao lado do
  FPGA, com a placa FPGA ligada nas Preferências: abre a aba e gira
  enquanto a placa está ligada.

### Corrigido

- Os layouts prontos listam a vista Assistente: sem ela, o Padrão aparecia
  sempre como modificado.
- **Ligar a placa** grava os arquivos abertos antes, como compilar e
  simular: a placa compilava o que estava no disco, e não o que estava na
  tela.
- A saída do projeto na placa virtual chega em lotes, com a contagem das
  linhas omitidas; um `$display` a cada ciclo travava a janela.
- A serial da placa virtual mostra a velocidade detectada quando o projeto
  não transmite na da placa, e lê os bytes como UTF-8 (o acento saía
  quebrado).
- A barra de ferramentas clássica fica só com os ícones quando os rótulos
  não cabem: com a placa FPGA ligada ela precisa de uns 1100 px, e numa
  janela de 960 px os últimos botões eram cortados. A dica de cada botão
  continua com o nome.

## [8.0.0] - 2026-11-05

### Alterado

- O Lace Studio passa a se chamar **AURORA IDE**: no título da janela, no
  splash, na tela inicial, em Sobre, nos menus e nos avisos. O logo é o
  camaleão do ícone da AURORA, com o mesmo contorno e em cor chapada: na
  barra de menus, na cor de destaque do tema; na tela inicial, ao lado do
  nome, apagado como marca d'água. Os ícones do aplicativo são o camaleão
  azul num quadrado grafite.
- O executável passa a ser `aurora-ide`, o `productName`, AURORA IDE (no
  macOS, `AURORA IDE.app`), e o identificador, `com.nipscern.aurora-ide`.
  Na primeira abertura, a IDE copia o `settings.json` do Lace Studio da
  pasta antiga (`com.nipscern.lace-studio`), com as preferências, os
  layouts e os recentes. As chaves do localStorage passam a começar por
  `aurora-ide:`: a sessão de cada projeto e o splash começam do zero.
- A IDE abre o `aurora` como servidor, pelo protocolo 2. A variável de
  desenvolvimento passa a ser `AURORA_IDE_CLI`, e a preferência, Programa
  aurora (`aurora_path`; o `lace_path` antigo vale).
- Cinco temas novos, cada um com o seu claro: **Boreal**, **Austral**,
  **Nebulosa**, **Alvorada** e **Lunar**, com o azul da aurora boreal, o
  verde da austral, o violeta das nebulosas, o âmbar do amanhecer e o
  prata da lua. O verde e o violeta são os da Aurora antiga, separados:
  no Austral, o lilás dela vai nos números e textos; no Nebulosa, a
  menta. Têm os mesmos cinzas quase neutros e uma cor de destaque; no
  código, só as palavras reservadas levam o destaque, e números e textos
  levam uma segunda cor. O Lunar deixa a interface no prata do texto e
  pinta o código com cores de luar: azul nas palavras reservadas, lilás
  nos tipos e dourado nos números e textos. O Boreal é o padrão.
- O tema passa a ser uma paleta e um modo. Em Preferências > Aparência, o
  modo (Escuro, Claro ou Do sistema) fica à parte, e cada paleta tem um
  cartão só, que serve aos dois esquemas; Exibir > Alternar tema claro e
  escuro troca o modo. O Dark Modern e o Light Modern viram a paleta VS
  Code; o Dracula e o Aurora Legacy são só escuros e ficam escuros em
  qualquer modo. Saem os temas de fora que não são do VS Code nem o
  Dracula (Gruvbox Dark e Light, Monokai, Nord, One Dark Pro, Solarized
  Dark e Light, Catppuccin Mocha e Tokyo Night): quem estava num deles
  passa para o Boreal, no mesmo esquema.
- O Atlas e o Atlas Branco, os padrões do Lace Studio, saem da lista:
  quem estava neles passa para o Boreal, escuro ou claro.
- O splash mostra o camaleão numa volta, a partir do focinho.
- O Studio passou a ser cliente do servidor do Lace (`lace serve`), como a
  extensão do VS Code: abre o `lace` como processo filho e pede a ele o
  projeto, os fluxos, o histórico, a placa FPGA, os exercícios, instalar e
  atualizar. Não linka mais o core, o `lace-learn` e o instalador; o único
  crate do Lace que ele linka é o do assistente, até a 1.0. Para quem usa,
  nada muda de lugar.
- Preferências > Bundle do Lace ganha **Programa lace**: o `lace` que o
  Studio abre como servidor. Vazio, o da instalação; para desenvolvimento,
  um build do repositório (ou a variável `LACE_STUDIO_LACE`).
- O seletor de sinais confere os caminhos na árvore do testbench antes de
  gravar a escolha, e a onda de um arquivo de exercício abre com o layout da
  correção também pelo menu Abrir a onda.

### Acrescentado

- A tela inicial ganha uma aurora no pé: três senoides finas que correm
  devagar, uma para cada lado, com as pontas apagadas. As cores são as da
  aurora da AURORA 7 (menta, azul e violeta) no Boreal e nos temas de fora,
  os verdes no Austral, o violeta e o rosa no Nebulosa, as do amanhecer no
  Alvorada e as da lua no Lunar. Com o sistema pedindo
  menos movimento, as linhas ficam paradas.
- Menus, diálogos, avisos e a vista que troca numa região aberta entram com
  um esmaecer curto, de 120 a 200 ms. Com o sistema pedindo menos
  movimento, nada anima.
- Preferências > Aparência ganha **Interface**: a Clássica, o padrão, ou
  Ilhas, com as barras laterais, o editor e o painel em ilhas de cantos
  redondos separadas por vãos, as abas em pílula e o ícone ativo da barra de
  atividades num quadrado tingido do destaque. Vale com qualquer tema.
- Os arquivos do fluxo da AURORA ganham ícones próprios no explorador e
  nas abas, cada tipo na cor dele no tema: Verilog, testbench, C±,
  assembly, MIF, onda e o `.spf`.
- **Exercícios**: a correção mostra a primeira amostra errada numa tabela
  (as entradas, e cada saída com o esperado e o seu, a que errou em
  destaque), os avisos do lint do Verilator sobre o seu arquivo e o latch
  que a síntese achou, com um link para a linha. O console da correção
  também escreve a primeira amostra errada.
- **Exercícios**: 26 exercícios novos na trilha de Verilog, em cinco
  capítulos (combinacionais, circuitos com memória, máquinas de estado,
  aritmética e memórias).

### Corrigido

- Um link `arquivo:linha` do INFO e dos terminais de etapa com espaço,
  acento ou símbolo no caminho (`C:\Users\Maria Silva\...`,
  `Área de Trabalho`) abria outro arquivo, ou nenhum: o link pegava só o
  trecho depois do último espaço. Agora o caminho absoluto vai inteiro,
  também quando o terminal o quebra em duas linhas, e um caminho relativo é
  procurado na raiz do projeto e na pasta onde a ferramenta rodou.
- Com a pasta do projeto movida, renomeada ou apagada fora do Studio, o
  aviso dizia que o `.spf` tinha sido apagado e oferecia criar um projeto
  novo no lugar antigo, vazio. Agora diz que a pasta não existe mais e
  oferece abrir o projeto no lugar novo. E o Studio confere o projeto quando
  a janela volta a ter o foco, e não só na próxima operação.
- Simular, sintetizar, ver a hierarquia e escolher sinais num projeto em
  pasta com acento ou outro alfabeto: o Lace recusava a simulação, e a
  síntese e a hierarquia falhavam.

### Segurança

- O editor usa o DOMPurify 3.4.17, que corrige falhas de XSS da 3.4.15. O
  Monaco 0.57 traz a 3.4.15 dentro dele; a IDE passa a usar no lugar dela
  o pacote corrigido.
- O Vite vai para a 8.3.5, que corrige falhas do servidor de
  desenvolvimento.

## [0.9.0] - 2026-10-20

### Acrescentado

- **Assistente**: uma conversa com o Claude Code sobre o projeto aberto, na
  barra lateral da direita (Exibir > Assistente) ou numa aba (Exibir >
  Assistente em aba). O assistente usa as ferramentas do Lace (compilar,
  verificar, simular, sintetizar, ler relatórios) e mostra cada uma que
  chamou, com o resultado; a resposta chega enquanto é escrita, e Esc a
  interrompe. Precisa do Claude Code instalado na máquina, com o login da
  conta ou a chave dele: o Studio não guarda chave. Preferências >
  Assistente: o executável do Claude Code, o modelo, se ele pode editar os
  arquivos do projeto, e o `lace` das ferramentas. É um recurso
  experimental, desligado por padrão: ligue em Preferências >
  Experimental > **Assistente de IA** (ou `lace experimental enable
  assistant`); desligado, a vista e a página de Preferências dele nem
  aparecem.

### Alterado

- Os fluxos (compilar, verificar, simular, sintetizar, esquemático, placa)
  são os do Lace (`lace_core::flows`), e não mais uma cópia da CLI dentro
  do Studio: a mesma regra nos dois, com o mesmo resultado. A entrega da
  saída ao vivo em lotes e o teto de linhas também vêm do Lace. Nada muda
  na tela; a fase `hierarchy` e a `rtl_viewer` aparecem no console como as
  outras.
- Instalar componentes e atualizar o Lace rodam a biblioteca do instalador
  dentro do Studio, sem rodar o `lace` como processo filho. A busca de
  atualização tem prazo por consulta; o console mostra o progresso como
  antes, e as linhas de erro trazem o código do instalador.
- A busca do bundle é a do Lace (`lace_core::locate`), com a ordem de
  antes; a tela de ferramentas vem do mesmo lugar que o `lace tools`.

## [0.8.0] - 2026-10-15

### Acrescentado

- **Remover processador**, no botão direito do processador no explorador e
  em Projeto > Remover processador (que pergunta qual). A confirmação
  oferece tirar só do projeto, com a pasta ficando no disco, ou tirar e
  mandar a pasta para a lixeira. O Verilog e o testbench gerados saem das
  listas de arquivos, e a aba do processador fecha. É o `lace proc remove`.
- A página **Experimental** nas Preferências, com os recursos do Lace que
  ainda mudam muito, desligados por padrão. Hoje há um, a **Placa FPGA**.
  O que se liga ali vale também para a CLI (`lace experimental`), e o
  contrário: os dois leem o `experimental.json` do Lace, cujo caminho a
  página mostra. Um `lace experimental enable` dado num terminal fora do
  Studio vale quando a janela volta a ter o foco.
- Na aba Fontes, em Fora do projeto, um **+** ao lado de cada arquivo, que
  aparece ao passar o ponteiro, o registra no projeto sem passar pelo menu
  de contexto, e o botão de lista com **+** no cabeçalho da seção registra
  todos. Adicionar vários arquivos de uma vez mostra um aviso só, com
  quantos foram para Módulos e para Testbenches.
- A busca de atualização automática: uns segundos depois de abrir e a cada
  12 horas, o Studio roda o `lace update --check` e, havendo Lace mais
  novo, oferece **Atualizar e reiniciar** num diálogo. O indicador **Lace X
  disponível**, na barra de status, fica até a atualização. Preferências >
  Geral > Procurar atualizações desliga a busca.
- Depois da atualização, o Studio grava os arquivos abertos e a sessão e
  reinicia sozinho na versão nova; ao abrir, um aviso diz a versão. No
  Windows, quando a atualização cai no assistente de instalação, o Studio
  fecha e o assistente o abre de novo no fim.
- O aviso de `.spf` apagado: se o arquivo do projeto aberto some, o Studio
  avisa que sem ele não há operação de projeto, desabilita compilar,
  simular, sintetizar e mexer nas fontes, e põe no alto do Explorador o
  botão **Criar um novo projeto**, com a pasta e o nome deste projeto. A
  árvore de arquivos, as abas e salvar continuam, e o projeto volta sozinho
  se o `.spf` reaparecer. Antes, cada operação dava um erro de projeto
  inválido, e o vigia de arquivos repetia o erro a cada arquivo salvo.
- A aba Placa FPGA conhece a Terasic Cyclone V GX Starter Kit (C5G). HEX2
  e HEX3 dividem os pinos com o conector GPIO e só acendem com as chaves
  DIP S1 e S2, no verso da placa, em ON.
- O **INFO**, no painel ao lado do Terminal: a saída de todas as operações
  num lugar só, uma rodada por operação. A rodada começa com a operação e a
  hora e termina com o status, o tempo e quantos erros e avisos ficaram em
  Problemas. A saída de cada ferramenta vem recuada embaixo do nome dela, e
  a do testbench embaixo de "saída do testbench". Ctrl+Shift+U traz o INFO
  para a frente.
- O botão de escolher o terminal, no cabeçalho de qualquer terminal, abre o
  INFO, o Terminal ou um terminal de etapa. Um terminal aberto fica aberto
  nas próximas sessões até ser fechado pelo X do cabeçalho.
- Preferências > Layout da janela escolhe o **modelo da barra de
  ferramentas**, que passa a fazer parte de cada layout: a **clássica**,
  numa linha própria; o **Executar na barra de título**, com o Alvo e o
  Parar; e o **Executar no editor**, os mesmos controles no canto das
  abas, que somem quando não há aba aberta. A seta do Executar abre o menu
  com todas as etapas e os atalhos, e a etapa escolhida ali passa a ser a do
  botão; no editor, com o fonte C± ou C++ de um processador na frente, o
  botão sugere compilar esse processador.
  Nos dois modelos novos, o editor ganha a altura da barra. O botão direito
  na barra também troca o modelo, e a lista do que ela esconde é a do
  modelo em uso.
- Dois layouts prontos, **Executar na barra de título** e **Executar no
  editor**. O segundo também põe o Navegador de fluxo na barra lateral
  direita.
- O tour pela interface: na primeira vez que o Studio abre, a janela
  escurece e uma parte de cada vez fica acesa, com um cartão que diz o que
  ela faz. A parte acesa continua funcionando, e alguns passos pedem uma
  tarefa (abrir o menu Fluxo, trocar de vista, esconder e mostrar o painel,
  abrir a paleta) e seguem sozinhos quando ela é feita. O tour volta por
  Ajuda > Tour pela interface e pela tela inicial. Um `settings.json` de
  versão anterior conta como tour já visto.
- O splash: enquanto o Studio abre, uma janela pequena mostra o logo do
  Lace sendo desenhado traço a traço, com um sinal que corre pelos fios, e o
  que está carregando. A janela principal só aparece pronta, com o Lace
  achado e o último projeto reaberto (no máximo 8 segundos), e o splash
  fecha; ele fica pelo menos 3,2 segundos, o tempo do desenho e de uma volta
  do sinal. Ele nasce nas cores e no idioma da abertura anterior.

### Mudado

- A placa FPGA virou recurso experimental e vem desligada. Desligada, ela
  sai da janela, não fica só cinza: a aba Placa FPGA (desligar com ela
  aberta a fecha), o botão FPGA da barra de ferramentas, as ações do menu
  Fluxo e da paleta, a seção FPGA do Fluxo, o terminal Placa (no painel,
  no seletor e no menu Exibir) e o Quartus Prime nas Preferências e nas
  Ferramentas do Lace. O backend recusa os comandos e as operações da
  placa com `experimental_disabled`. Liga em Preferências > Experimental.
- Na barra de ferramentas clássica, um separador fica entre os grupos do
  fluxo: compilar, simular, a onda e o hardware.
- No fim da barra de status, o nome do layout em uso dá lugar a três
  botões, como os do canto da barra de título do VS Code: mostrar e
  esconder a barra lateral esquerda, o painel e a barra lateral direita. O
  layout se troca por Ctrl+K L, pelo menu Exibir > Layout ou pelas
  Preferências.
- Os seis consoles de etapa (C±, ASM, Verilog, Wave, RTLViewer e Placa, que
  correspondem aos terminais da AURORA) começam fechados. Aberto pelo seletor
  ou por Exibir > Consoles, um deles mostra o que a etapa já tinha escrito e
  passa a receber as linhas novas dela, que também vão para o INFO. Um
  layout salvo antes disso ganha o INFO no lugar do primeiro console, com o
  Terminal ao lado.
- A cor dos consoles só mostra como algo terminou (✓ verde, ✗ vermelho, !
  amarelo) e a palavra de gravidade nas mensagens das ferramentas. Antes, uma
  linha inteira mudava de cor por palavra-chave, e o stderr saía amarelo
  mesmo sem aviso nenhum.
- O painel não troca mais de aba no meio de uma operação: o INFO vem para a
  frente no começo e, numa falha, rola até o primeiro erro.
- A estrutura de pastas do YANC 7.1, a mesma da AURORA: o processador novo
  é a pasta `<proc>/` com o fonte, sem `#PRNAME` e sem as pilhas que
  ninguém escolheu (o campo vazio diz "pelo programa"); o arquivo de
  entrada novo fica ao lado do fonte (`<proc>/input_<n>.txt`); o que o YANC
  gera, com as saídas da simulação, fica em `<proc>/<proc>/`. A aba
  Toolchain não lista mais o `appcomp` e o `cpppp`, que a 7.1 não tem.
- Ao abrir um projeto com `Software/`, `Hardware/` e `Simulation/`, o
  Studio pergunta se pode converter para a estrutura nova (nada é apagado
  nem sobrescrito) e depois mostra o que mudou de lugar e o que ficou. A
  conversão também fica em Arquivo > Converter projeto para o YANC 7.1, e o
  painel de problemas avisa até lá.
- Fechar o projeto, abrir um projeto sem abas guardadas e abrir o Studio
  sem projeto não abrem mais a aba **Lace Studio**: o editor vazio já mostra
  a mesma tela.
- No menu Arquivo, os projetos recentes ficam no submenu **Abrir recente**,
  que abre ao lado com o mouse em cima, em vez de ocupar o menu inteiro.
- Com as Preferências ou o RTLViewer na frente, as barras laterais e o
  painel se escondem, e voltam como estavam quando outra aba vem para a
  frente. O layout salvo não muda com isso.
- Sem arquivo na frente (outra vista, o editor vazio), o Ctrl+F abre a Busca
  do projeto; com o RTLViewer na frente, a busca dele.

### Corrigido

- Um layout pronto, o Padrão inclusive, aparecia como modificado (o ponto
  depois do nome, na barra de status) logo depois de escolhido: a vista
  Exercícios não estava na lista dele e entrava sozinha no fim da barra
  lateral.
- Com o modo Vim ligado, o Ctrl+F no editor descia uma página (o Ctrl+F do
  Vim) em vez de abrir a busca no arquivo.

## [0.7.0] - 2026-10-12

### Acrescentado

- **Escolher os sinais da onda**: a árvore do testbench do projeto, com uma
  caixa por escopo e por sinal, filtro por nome ou expressão regular, **Só o
  testbench** e **Desmarcar tudo**. A escolha vai para
  `wave/<testbench>.json`; a próxima simulação grava só ela, e a onda mostra
  só ela, um grupo por escopo. **Gravar e simular** já roda a Wave do
  projeto com a escolha nova. Abre pelo botão **Sinais** da barra de
  ferramentas, ao lado de Onda, que mostra quantos itens a escolha tem; pelo
  botão Sinais no alto da aba de onda; pelo Navegador de fluxo; pelo menu
  Fluxo; e pelo menu do testbench simulado no Explorer.
- O layout da onda salvo no projeto, `wave/<testbench>.surf.ron`: o Ctrl+S
  dentro da aba de onda (que agora fica com o Surfer) e o da janela salvam
  nele, e a onda reabre com ele. O alto da aba mostra onde ele está e, depois
  que você salva, **Voltar ao layout gerado**.

### Mudado

- O **Fast Sim** ficou mais rápido num testbench que imprime muito: o
  modelo do Verilator não força mais o flush de cada `$display`, como na
  AURORA, e a saída chega ao console em blocos. Cancelado, ele perde as
  últimas linhas que ainda estavam no buffer.
- Depois de compilar um processador (o build sozinho, ou o que a Wave, o
  Synth e a compilação para a placa fazem antes), o `Hardware/<nome>.v` e o
  `Simulation/<nome>_tb.v` que ele gerou entram no projeto, em Módulos e em
  Testbenches, como na AURORA, em vez de só aparecerem como gerados. Num
  projeto sem módulos, o do processador vira o topo; sem testbench, o dele
  vira o simulado. Quem faz é o Core (`Project::register_generated`).
- O RTLViewer vem do pacote `@lace/rtl-viewer` (`packages/rtl-viewer/` do
  repositório do Lace), o mesmo que a extensão do VS Code e o
  `lace synth --open` mostram. Nada muda no uso: a aba, os atalhos, as
  cores e as preferências guardadas são os de antes.
- **Atualizar para X**, na tela de ferramentas, usa a atualização por
  componentes do `lace update`: só os pedaços com arquivos que mudaram são
  baixados, também no Windows, onde o assistente de instalação só abre se
  ela não der certo. A tela mostra cada pedaço enquanto baixa e, no fim, o
  tamanho baixado e o que mudou; quando o próprio Studio muda, avisa para
  fechar e abrir de novo.
- Novo projeto e Abrir projeto começam na última pasta usada ou, na
  primeira vez, em `Projetos` na pasta do usuário (`~/Projetos` no Linux e
  no macOS, `C:\Users\<nome>\Projetos` no Windows), que o Studio cria se
  falta. Antes o Novo projeto começava na pasta do usuário.

## [0.6.0] - 2026-10-09

### Acrescentado

- Preferências > Bundle do Lace > **Quartus Prime**: a pasta do Quartus,
  que compila para as placas Intel (o `--quartus` da CLI). Vazia, o Studio
  usa o `LACE_QUARTUS`, o `QUARTUS_ROOTDIR` e as pastas padrão do
  instalador. A tela de ferramentas mostra o Quartus encontrado e a versão.
- A aba **Placa FPGA** (o botão **Placa** na barra de ferramentas, o menu
  Fluxo ou a paleta de comandos) leva o projeto para a placa em três etapas,
  cada uma com o estado: **Ligações**, **Compilação** e **Gravação**. As
  ligações vêm das portas do topo, uma linha por porta, com um painel dos
  sinais da placa que cabem nela, agrupados em clocks, botões, chaves, LEDs
  e displays, os bits e a caixa **Inverter**; o
  `fpga.json` é gravado e conferido a cada mudança, e cada problema aparece
  na linha da porta. **Ligar automaticamente** liga as portas pelos nomes
  iguais aos da placa e, num processador SAPHO, o `clk` no oscilador, o
  `rst` no primeiro botão (invertido), a entrada nas chaves e as saídas nos
  LEDs. **Compilar e gravar na placa** roda o `lace fpga build` e, se der
  certo, o `lace fpga program`; a etapa de compilação diz quando o arquivo
  de gravação ficou desatualizado e por quê, e a de gravação diz se o cabo
  da placa está ligado. No fim aparecem os recursos da FPGA e a Fmax e as
  folgas de cada clock. A saída do Quartus vai para o console novo **Placa**,
  e os erros, para Problemas.

### Corrigido

- O testbench que o build de um processador gera não tinha, no menu da aba
  Fontes, **Marcar como o testbench simulado**; só dava para marcá-lo
  depois de adicioná-lo pela aba Arquivos. Agora o item aparece nele e faz
  os dois passos: registra no `.spf` como testbench e marca como o
  simulado.

### Mudado

- O **+** de Módulos, Testbenches e Processadores SAPHO, na aba Fontes,
  fica sempre à vista, e não só com o mouse sobre o cabeçalho da seção.

## [0.5.0] - 2026-10-08

### Acrescentado

- A vista Exercícios, do `lace learn` (componente `lace-learn`): cria ou
  abre a pasta de exercícios e lista os capítulos com o que já foi
  resolvido. Escolher um exercício abre o projeto dele, o arquivo à
  esquerda e o enunciado à direita, sem corrigir; cada gravação corrige, e
  Ctrl+Alt+L corrige quando quiser. A aba do enunciado mostra as dicas
  pedidas, a correção (cada saída errada, o primeiro erro, os erros de
  compilação, que levam à linha) e os botões da onda, do esquemático, de
  restaurar, da solução e do próximo exercício. A correção é a operação
  `learn`: os erros vão para a aba do enunciado, para Problemas e para o
  editor.
- A sessão de exercícios: com um exercício aberto, trocar de aba não grava
  sozinho e o painel de baixo fica escondido até ser chamado; nenhuma
  operação o abre. Ao sair da sessão, o painel volta como estava.
- A onda de um exercício abre na aba (ou na janela) com as entradas, as
  saídas ao lado das da referência e o primeiro erro marcado.
- Preferências > Bundle do Lace: a pasta das trilhas de exercícios, para
  quem escreve exercícios testar uma trilha sem instalar o componente. Sem
  o componente, a vista Exercícios oferece instalá-lo ou escolher essa
  pasta.

## [0.4.0] - 2026-10-07

### Acrescentado

- Testbench cocotb no Verilator: com o Verilator escolhido, a Wave e a Rápida
  rodam os testes nele, e a onda sai em `.vcd`.
- Layout da janela. Há uma barra lateral direita, e o painel pode ficar à
  direita do editor. Cada vista (Explorador, Fluxo, Busca, Relatórios, os
  consoles, Problemas e Terminal) vai para qualquer região pelo botão
  direito > **Mover para**. Também pelo botão direito, cada vista, barra,
  botão da barra de ferramentas e item da barra de status pode ser
  escondido. **Exibir > Aparência** liga e desliga as barras e as regiões.
  A barra de atividades fica à esquerda, à direita ou escondida.
- Layouts com nome: **Exibir > Layout** salva a janela como um layout,
  restaura, troca (Ctrl+K L ou o nome do layout na barra de status) e volta
  ao Padrão, que é a janela de antes. **Preferências > Layout da janela**
  junta tudo numa tela, com renomear e excluir. Os layouts ficam no
  `settings.json`.
- Os tamanhos das barras laterais e do painel ficam guardados. Duplo clique
  numa divisão volta ao tamanho do layout em uso.
- **Fluxo > Escolher o alvo** e **Exibir > Consoles**, para quando o
  seletor de alvo ou um console estiver escondido.
- Ctrl+Alt+B mostra e esconde a barra lateral direita.
- Ctrl+K T abre a lista de temas (Exibir > Selecionar tema), como o
  Ctrl+K Ctrl+T do VS Code.
- Barra de título integrada: os menus, o nome do projeto e os botões de
  minimizar, maximizar e fechar numa faixa só, no lugar da barra de título
  do sistema mais a barra de menus. Arrastar a faixa move a janela, e dois
  cliques maximizam. No macOS, os botões coloridos do sistema ficam no
  canto da faixa. No Windows 11, o menu de encaixe que aparece sobre o
  maximizar não existe nessa faixa; Win+Z e arrastar até a borda
  continuam.

### Mudado

- O RTLViewer (F10) tem esquemático próprio: o Studio desenha o netlist da
  síntese em vez de mostrar a imagem do Graphviz. As cores seguem o tema,
  com um tom por família de célula (aritmética, lógica, comparação,
  multiplexador, registrador, memória, submódulo) e a legenda no rodapé.
  No uso é o RTLViewer da AURORA: entrar no submódulo (dois cliques ou o ícone
  da caixa), voltar com Esc, Alt+← ou o botão lateral do mouse, ir ao
  código com dois cliques num símbolo, destacar as ligações, exportar o
  SVG. A mais: árvore de hierarquia, trilha do caminho, busca (Ctrl+F),
  painel de detalhes com parâmetros, código e a rede de cada porta,
  minimapa, e o destaque segue a rede no netlist (com as fatias), não a
  geometria dos fios. Constantes e redes globais (`clk`, `rst`) aparecem
  escritas na porta, sem fio. O layout roda num worker e um módulo grande
  não trava a janela: o `ula_fdiv` do proc_fft leva 1,4 s, o `fir` com
  32 taps, 0,13 s. A síntese do Studio não roda mais o Graphviz; a CLI
  continua com ele (`lace synth --svg`).
- A Rápida (F9) é a simulação rápida, o Fast Sim da AURORA: roda sem gravar
  onda (`lace sim --fast`). O testbench Verilog e o de um processador rodam
  no Verilator, qualquer que seja o simulador escolhido; um testbench cocotb
  roda os testes no simulador escolhido. Antes, a Rápida era a Wave sem
  abrir a onda. O navegador de fluxo mostra o simulador que ela usa, o
  console e a barra de status a chamam de simulação rápida, e a página do
  processador ganhou o botão.
- O item do Explorer que marca o testbench só marca: chama-se **Marcar como
  o testbench simulado** (era **Simular este testbench**), e sai do menu o
  item de simulação rápida que trocava o testbench e já simulava. Quem
  simula é a Wave ou a Rápida.
- O menu Exibir tem os submenus Consoles, Aparência e Layout. Mostrar ou
  ocultar a barra lateral e o painel foi para Aparência.
- O terminal e a Busca só pegam o foco quando são pedidos. Aparecer porque
  um layout foi aplicado não tira o foco do editor.
- As Preferências têm uma página por assunto (Geral, Aparência, Layout da
  janela, Editor, Simulação, Bundle do Lace), com a lista à esquerda, em
  vez de uma página só com tudo. Cada preferência é uma linha, com o
  controle à direita: interruptor para ligar e desligar, escolha à vista
  para duas ou três opções. O shell do terminal foi para Geral e o Modo zen,
  para Editor. A página aberta é lembrada.
- O tema Atlas ficou mais vivo. O destaque passou do azul do CERN ao azul
  do ATLAS (#0B80C3, a cor da identidade visual do experimento), os fundos
  ganharam um tom de ardósia puxado para esse azul, com o editor um tom
  abaixo da barra lateral, e a sintaxe tem um tom por papel, com as
  palavras reservadas no azul do ATLAS e `module` e `endmodule` em negrito,
  num azul mais vivo. O Atlas Branco continua no azul do CERN.
- O assembly do editor conhece as leituras da divisão do YANC 6.0
  (`QUO`, `REM`, `F_QUO`), e a ajuda de `DIV`, `MOD` e `F_DIV` mostra a
  sequência de três palavras que o `asmcomp` passou a exigir.

### Corrigido

- O menu do botão direito do editor saía transparente, com o texto preto,
  em todos os temas. Agora segue o tema, como os menus do Studio.
- No Verilog, `module` e `endmodule`, `begin` e `end` saíam na cor dos
  operadores, pintados como pares de parênteses. Agora têm a cor das
  palavras reservadas, e só parênteses, colchetes e chaves ganham cor de
  par.

## [0.3.0] - 2026-10-06

### Acrescentado

- Testbench em Python, com o cocotb: **Projeto > Novo testbench cocotb
  (Python)**, ou a linguagem Python em **Novo testbench**, cria um `.py` a
  partir do modelo do Core, com a linha `# aurora-toplevel:` do módulo
  testado; um `.py` que já existe entra por **Adicionar arquivos** ou
  arrastado para Testbenches, e os da pasta com `@cocotb.test` aparecem
  entre os não registrados. Com ele como testbench simulado, Wave (F8) e
  Rápida (F9) rodam os testes no Icarus: cada teste aparece no console Wave
  com o resultado, o que falhou vai para o painel Problemas na linha do
  `.py`, e a onda abre mesmo com teste falhando.

### Mudado

- O terminal acompanha o projeto: ao abrir outro, o shell que está rodando
  entra na pasta dele (`Set-Location` no PowerShell, `cd` nos outros).
- A aba Lace saiu do painel inferior. A saída do `lace install` e do
  `lace update` aparece na tela de Ferramentas, que abre quando o comando
  começa; o início de cada fluxo vai para o console da etapa.
- O Verilog (F7) não compila mais os processadores: verifica o Verilog que
  está no disco, e um processador que nunca foi compilado fica de fora, com
  um aviso. O fluxo completo (F5) compila antes de verificar.
- O esquemático não recusa mais um módulo por ter muitas ligações.
- A onda do Icarus sai sempre em FST, e o layout dos processadores vale
  também para ela.

## [0.2.0] - 2026-10-05

A primeira versão com o Studio, junto com a 0.2.0 do Lace.

### Distribuição

- O Studio passa a morar no repositório do Lace, em `studio/`, e a sair no
  instalador dele como o componente `studio`, marcado na instalação
  Recommended e com `lace install studio` para depois. Fica no bundle
  (`toolchain/studio/`) e ganha atalho no menu de aplicativos: o
  `lace-studio.desktop` no Linux, o `Lace Studio.app` em `~/Applications` no
  macOS, o menu Iniciar no Windows.
- Instalado no bundle, o Studio usa o bundle em que está, antes da pasta
  padrão da instalação (origem `bundled`, "bundle do Studio" na tela de
  ferramentas): uma instalação em outra pasta não abre o bundle de outra.

### Acrescentado

- Janela de IDE: menus, barra de ferramentas com os botões da AURORA (C±,
  Verilog, Wave, Rápida, RTLViewer, Parar), barra de atividades, barra lateral,
  abas, painel inferior e barra de status, com divisões redimensionáveis.
- Projeto: criar, abrir (inclusive arrastando o `.spf`), fechar, recentes,
  reabrir o último projeto com as abas de cada um.
- Explorador com a vista de fontes (módulos com o topo, testbenches com o
  simulado, processadores com fonte, entradas, saídas e gerados, arquivos
  fora do projeto) e a vista de pastas (criar, renomear, lixeira).
- Processadores SAPHO: criar (os campos do Processor Hub da AURORA),
  configurar frequência, clocks e arrays, entradas e saídas.
- Fluxos do Lace com saída ao vivo e cancelamento: build, check (e lint),
  simulação do projeto ou de um processador (Icarus ou Verilator, com prazo),
  síntese, esquemático, abrir a onda no surfer-aurora, fluxo completo (F5).
- Consoles por etapa (C±, ASM, Verilog, Wave, RTLViewer, Lace) com links
  `arquivo:linha`, painel Problemas e marcadores no editor.
- Terminal de shell com o `lace` no `PATH`.
- Editor Monaco com C± e assembly do SAPHO (gramáticas portadas da AURORA e
  corrigidas pelo léxico do YANC), Verilog, SystemVerilog e JSON;
  sugestões de C±; detecção de mudança no disco.
- Vistas: boas-vindas, esquemático com zoom, estatísticas da síntese,
  relatórios e comparação, ferramentas do Lace (componentes, executáveis,
  hashes, `lace install`, `lace update --check`), processador, preferências.
- Navegador de fluxo no estilo do Vivado, busca nos arquivos, paleta de
  comandos, abrir arquivo pelo nome.
- Português e inglês.
- Temas, em Preferências > Aparência (um cartão com a prévia de cada um) ou
  Exibir > Selecionar tema: Atlas (o padrão, neutro, com o azul do CERN),
  Atlas Branco, Aurora Legacy (as cores da AURORA), Dark Modern, Light
  Modern, Dracula, Gruvbox Dark e Light, Monokai, Nord, One Dark Pro,
  Solarized Dark e Light, Catppuccin Mocha e Tokyo Night, ou "Do sistema".
  Cada tema pinta a interface, o editor (inclusive C± e asm) e os consoles.
  Quem tinha "escuro" ou "claro" gravado passa ao Atlas ou ao Atlas Branco.
- Modo zen (Ctrl+K Z, Exibir > Modo zen): só o editor, em tela cheia e numa
  coluna centralizada. Ctrl+` abre o terminal numa gaveta embaixo do
  editor; um indicador no canto mostra a operação rodando e o resultado.
  Sai com Ctrl+K Z ou Esc duas vezes, e o layout volta como estava. As
  opções ficam em Preferências > Modo zen.
- Atalhos de duas etapas, como no VS Code (Ctrl+K Z), que convivem com os
  do Monaco (Ctrl+K Ctrl+C continua comentando a linha).
- Documentação: guia do usuário, arquitetura, referência do IPC,
  desenvolvimento e paridade com a AURORA.
- Testes do backend, inclusive dos fluxos contra o bundle instalado.
- Hierarquia do design no explorador (Fontes, Hierarquia, Arquivos): a
  árvore de instâncias do design e de cada testbench, elaborada pelo Icarus,
  com a biblioteca SAPHO, os blocos `generate` resolvidos e o topo marcado.
  Atualiza sozinha depois de cada verificação, simulação, build ou síntese
  que passa, e avisa quando um `.v` mudou depois.
- Fontes: o Verilog gerado de cada processador aparece em Módulos e o
  testbench gerado em Testbenches, marcados como gerados. Cada processador
  separa o programa (C± e assembly), as memórias, a simulação (entradas,
  saídas, onda) e os intermediários do YANC.
- Arrastar e soltar no explorador: mover arquivos e pastas na árvore de
  arquivos (o `.spf` acompanha, com o topo e o testbench escolhido), trocar
  um Verilog entre Módulos e Testbenches, soltar arquivos do sistema numa
  pasta (copia) ou numa seção das fontes (copia para o projeto e registra).
- Árvore: setas, Home e End para andar, direita e esquerda para abrir e
  fechar, o arquivo ativo revelado, as pastas abertas lembradas por projeto,
  marcas de topo, simulado e registrado na árvore de arquivos.
- Limpar relatórios (`lace report clean`): todos, ou todos menos os N mais
  novos, com a lista do que sai antes de apagar; apagar um relatório pelo
  menu dele.
- Modo Vim no editor (monaco-vim), com o modo na barra de status e `:w`,
  `:q`, `:wq`, `:x`, `:wa`.
- No Windows, o terminal pode ser o PowerShell ou o Prompt de Comando
  (Preferências > Terminal).
- Editor dividido: até três grupos lado a lado, cada um com as suas abas e
  o seu editor (`Ctrl+\`, "Dividir à direita", "Abrir ao lado" no
  explorador ou Ctrl+Enter na árvore); arrastar abas entre grupos ou para
  um grupo novo, Ctrl+1/2/3 para ir a um grupo; o mesmo arquivo em dois
  grupos é o mesmo texto. Os grupos voltam com o projeto.
- Explorador: um botão no título recolhe a árvore inteira e, de novo,
  expande tudo, nas Fontes, na Hierarquia e na árvore de arquivos.
- A onda de um processador SAPHO abre arrumada como na AURORA: um grupo por
  processador com I/O, a instrução de assembly e a linha do C± de cada
  ciclo, as variáveis do programa e as flags (o `wave_layout` do Core). Vale na
  aba e na janela.
- A onda numa aba: o cliente web do Surfer dentro do Studio, servido por um
  servidor local em `127.0.0.1`. Preferências > Simulação escolhe aba (o
  padrão) ou janela separada; simular de novo recarrega a aba aberta.
- Ferramentas do Lace: **Atualizar para X**, depois de procurar
  atualizações, quando há um Lace mais novo. Confirma e roda o
  `lace update --yes --json` da instalação como operação, com a saída do
  instalador no console Lace; no Windows, abre o assistente. Não pode ser
  cancelada no meio.
- O azul do CERN (Pantone 286 C, #0033A0) nos detalhes: foco, aba e item
  ativos, botão principal, seleção, destino de arrastar.

### Mudado

- O símbolo do Lace passou a ser a torção (dois fios que trocam de posição
  duas vezes) no lugar do octógono entrelaçado: barra de menus,
  boas-vindas, aba Sobre, favicon e ícones do app.
- As marcas do explorador começam com maiúscula: Topo, Simulado, Gerado,
  Compilado, Não compilado (Top, Simulated, Generated, Built, Not built).
- O seletor Icarus/Verilator saiu da barra de ferramentas; o simulador se
  escolhe no menu Fluxo ou nas Preferências.
- Sobre virou uma aba: versões do Studio, do bundle e de cada componente,
  do Tauri e do motor de páginas; as pastas das preferências e do bundle;
  links da documentação; "Copiar informações" para relatar um problema.

- O logo provisório (um L entre dois pinos) deu lugar à marca do Lace na
  barra de menus, nas boas-vindas, no Sobre, no favicon e nos ícones do app.
- Nome de projeto novo só com letras sem acento, números, `_` e `-`,
  começando por letra; o diálogo avisa enquanto se digita.
- As ações do explorador e dos relatórios ficam na linha do título da barra
  lateral.
- A hierarquia, mover arquivos com o `.spf` em dia e a regra de nome de
  projeto passaram para o Core do Lace (`lace_core::hierarchy`,
  `Project::move_path`, `validate_project_name`, com `lace hierarchy` e
  `lace move` na CLI); o Studio só os chama. A hierarquia ganhou o status e
  os erros de cada elaboração, com arquivo e linha.
- Renomear na árvore de arquivos (F2) passa pelo Core, como mover: um `.v`
  registrado continua no `.spf`.
- O Verilog (F7) e o lint seguem o alvo da barra de ferramentas: com um
  processador escolhido, compilam só ele e verificam o Verilog dele com o
  testbench que o YANC gerou (`lace check -p`), como os outros botões já
  faziam. "Verificar este arquivo" continua valendo para o projeto.
- Qualquer Verilog pode ser escolhido como topo, inclusive o `.v` gerado
  pelo build de um processador e os de fora do projeto, menos nome de
  testbench (`tb_<nome>.v`, `<nome>_tb.v`, `tb.v`, a regra do Core). O
  "Definir como topo" está nas Fontes (módulos, gerados, testbenches com
  outro nome, fora do projeto), na árvore de arquivos e na hierarquia, e o
  diálogo Escolher o topo lista todos os candidatos.
- `lace install` e `lace update` que falham mostram no console Lace e no
  aviso a mensagem que a CLI escreveu no JSON, não só o código de saída.
- O resultado de "Procurar atualizações" fica no alto da tela de
  ferramentas, logo abaixo da pasta do bundle.

### Corrigido

As pendências do teste de fogo de 2026-10-04 estão em `docs/PENDENCIAS.md`
(as do Core e da CLI, em `lace/docs/PENDENCIAS.md`).

- Arrastar um `.v` de fora da pasta do projeto para Módulos ou Testbenches
  copiava o arquivo para a raiz, e a cópia divergia do original. Agora
  registra no lugar, como o menu Adicionar; no mesmo repositório git, o
  `.spf` guarda o caminho relativo.
- Os avisos do `.spf` (caminho de outra máquina achado dentro da pasta, topo
  fora da lista, processador com nome que não compila) aparecem no painel
  Problemas enquanto o projeto está aberto.
- O diálogo de processador novo confere o nome com o Core enquanto o
  usuário digita (palavras do C± e do Verilog, módulos da biblioteca SAPHO,
  nomes longos), mostra o motivo ao passar o mouse e mantém o `#NUBITS`
  igual a `#NBMANT + #NBEXPO + 1`.
- A onda de um processador compilado de novo depois da simulação avisa, na
  aba e na janela, que o PC e a linha aparecem como números até simular de
  novo; antes, o assembly e o C± saíam deslocados sem aviso.
- Com o foco na aba da onda, os atalhos do Studio não funcionavam (o
  teclado era do Surfer até clicar fora). O cliente web passa ao Studio as
  teclas de função e as combinações com Ctrl, Alt ou Cmd; letras, setas,
  espaço e as de edição continuam no Surfer.
- Mudar a ordem dos arquivos das fontes (a ordem dos compiladores, que
  decide quem vê cada `define) exigia tirar e adicionar de novo: agora se
  arrasta um sobre outro da mesma seção, ou usa Subir e Descer na lista no
  menu (`lace order` do Lace).
- Os avisos novos do `.spf` (arquivo repetido nas listas, projeto dentro da
  pasta de outro) aparecem traduzidos no painel Problemas.
- Três códigos de erro novos do Core traduzidos: `ambiguous_module`,
  `invalid_parameter`, `operation_in_progress` (uma simulação do Studio e
  outra da CLI no mesmo processador não se quebram mais: a segunda recusa).

- Com muitas abas abertas, a janela inteira deslizava para o lado junto com
  elas (o explorador e os menus sumiam da tela). A coluna da grade da janela
  não cresce mais com o conteúdo, e a aba ativa entra na vista rolando só a
  barra de abas.
- Trocar de aba recarregava a onda do zero e perdia zoom, cursor e sinais
  acrescentados: as abas de onda ficam montadas enquanto existem.
- Fechar o projeto gravava a sessão vazia por cima das abas salvas; ao
  reabrir, só a aba de boas-vindas voltava.
- Abrir outro projeto mantinha os problemas, os marcadores e o "Último:" do
  anterior.
- Na vista Fontes, arquivo de fora da pasta do projeto (o HITS usa
  `../../rtl/`) aparecia com o caminho absoluto cortado; agora vai o nome,
  com a pasta relativa ao lado. O painel Problemas usa a mesma forma.
- 13 códigos de erro saíam crus, em inglês (`invalid_project`,
  `invalid_bundle`, os da aba de onda); todos têm texto agora, inclusive os
  novos `non_ascii_path` e `schematic_too_large` do Lace.
- Passo cancelado aparecia como "falhou ... (cancelled)", e a barra dizia
  "simulação cancelado": o passo diz "parou (a pedido)", e as palavras de
  status não dependem mais do gênero ("Simulação de x: terminou em ...").
- A Hierarquia de um projeto sem arquivo Verilog mostrava a mensagem crua do
  Lace como erro; agora mostra o estado vazio com "Adicionar arquivos
  Verilog".
- Em janela estreita, as abas do painel inferior passavam por baixo dos
  ícones e a barra lateral ocupava quase metade da largura (teto agora de
  40%).
- O RTLViewer de um módulo grande demais para o esquemático perdia também a
  síntese: agora a síntese e as estatísticas ficam, e o esquemático recusado
  vira aviso (`schematic_error`).

- A janela abria em branco e ficava assim até o JavaScript carregar (1,5 s
  a 4 s com `npm run tauri dev`, enquanto o Vite serve os módulos). Agora
  ela nasce escondida, com fundo escuro, e aparece já desenhada, no tema
  das preferências e com o último projeto aberto.

- A interface relia o projeto em ciclo: o vigia de arquivos avisava também
  quando um arquivo era só aberto para leitura, e reler o projeto abre o
  `.spf`. Agora só mudança (gravar, criar, apagar, renomear) avisa.
- O Surfer fechado ficava como processo zumbi até o Studio sair.
- Botões que davam erro em vez de ficar desabilitados: Lint e Verilator sem
  o Verilator instalado, Onda sem onda gerada, Último relatório sem
  relatórios (agora um aviso).
- Erros benignos do Monaco (promessas canceladas, leitura da área de
  transferência negada) não vão mais para o log como erro.
- O modo do Vim não aparecia na barra de status: o monaco-vim esconde o
  elemento que recebe ao criá-lo e não o mostra de novo.
